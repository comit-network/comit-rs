use crate::{
    db::CreatedSwap,
    identity,
    seed::{DeriveSwapSeed, RootSeed},
    swap_protocols::{
        hbit, herc20,
        rfc003::{DeriveSecret, SecretHash},
        Herc20HalightBitcoinCreateSwapParams, LocalSwapId, Role, SharedSwapId,
    },
    timestamp::Timestamp,
};
use ::comit::network::{
    oneshot_behaviour,
    protocols::{
        announce,
        announce::{behaviour::Announce, protocol::ReplySubstream},
        ethereum_identity, finalize, lightning_identity, secret_hash,
    },
};
use digest::Digest;
use libp2p::{
    swarm::{
        NegotiatedSubstream, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour, PeerId,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    task::{Context, Poll},
};
use swaps::Swaps;

/// Setting it at 5 minutes
const PENDING_SWAP_EXPIRY_SECS: u32 = 5 * 60;

mod swaps;

/// Event emitted  by the `ComitLn` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    SwapFinalized {
        local_swap_id: LocalSwapId,
        swap_params: Herc20HalightBitcoinCreateSwapParams,
        secret_hash: SecretHash,
        ethereum_identity: identity::Ethereum,
        lightning_identity: identity::Lightning,
    },
}

#[derive(NetworkBehaviour, Debug)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct Comit {
    announce: Announce,
    secret_hash: oneshot_behaviour::Behaviour<secret_hash::Message>,
    ethereum_identity: oneshot_behaviour::Behaviour<ethereum_identity::Message>,
    lightning_identity: oneshot_behaviour::Behaviour<lightning_identity::Message>,
    finalize: oneshot_behaviour::Behaviour<finalize::Message>,

    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    swaps: Swaps<ReplySubstream<NegotiatedSubstream>>,
    #[behaviour(ignore)]
    ethereum_identities: HashMap<SharedSwapId, identity::Ethereum>,
    #[behaviour(ignore)]
    lightning_identities: HashMap<SharedSwapId, identity::Lightning>,
    #[behaviour(ignore)]
    communication_state: HashMap<SharedSwapId, CommunicationState>,
    #[behaviour(ignore)]
    secret_hashes: HashMap<SharedSwapId, SecretHash>,

    #[behaviour(ignore)]
    pub seed: RootSeed,
}

#[derive(Debug, Default)]
struct CommunicationState {
    ethereum_identity_sent: bool,
    lightning_identity_sent: bool,
    received_finalized: bool,
    sent_finalized: bool,
    secret_hash_sent_or_received: bool,
}

impl Comit {
    pub fn new(seed: RootSeed) -> Self {
        Comit {
            announce: Default::default(),
            secret_hash: Default::default(),
            ethereum_identity: Default::default(),
            lightning_identity: Default::default(),
            finalize: Default::default(),
            events: VecDeque::new(),
            swaps: Default::default(),
            ethereum_identities: Default::default(),
            lightning_identities: Default::default(),
            communication_state: Default::default(),
            secret_hashes: Default::default(),
            seed,
        }
    }

    pub fn initiate_communication(
        &mut self,
        id: LocalSwapId,
        create_swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        let digest = create_swap_params.digest();
        tracing::trace!("Swap creation request received: {}", digest);

        match create_swap_params.role {
            Role::Alice => {
                tracing::info!("Starting announcement for swap: {}", digest);
                self.announce
                    .start_announce_protocol(digest.clone(), (&create_swap_params.peer).clone());
                self.swaps
                    .create_as_pending_confirmation(digest, id, create_swap_params)?;
            }
            Role::Bob => {
                if let Ok((shared_swap_id, peer, io)) = self
                    .swaps
                    .move_pending_creation_to_communicate(&digest, id, create_swap_params.clone())
                {
                    tracing::info!("Confirm & communicate for swap: {}", digest);
                    self.bob_communicate(peer, io, shared_swap_id, create_swap_params)
                } else {
                    self.swaps.create_as_pending_announcement(
                        digest.clone(),
                        id,
                        create_swap_params,
                    )?;
                    tracing::debug!("Swap {} waiting for announcement", digest);
                }
            }
        }

        Ok(())
    }

    pub fn init_hbit_herc20(
        &mut self,
        _: LocalSwapId,
        _: CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>,
    ) -> anyhow::Result<()> {
        // As for initiate_communication()
        unimplemented!()
    }

    pub fn init_herc20_hbit(
        &mut self,
        _: LocalSwapId,
        _: CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>,
    ) -> anyhow::Result<()> {
        // As for initiate_communication()
        unimplemented!()
    }

    pub fn get_created_swap(
        &self,
        swap_id: &LocalSwapId,
    ) -> Option<Herc20HalightBitcoinCreateSwapParams> {
        self.swaps.get_created_swap(swap_id)
    }

    /// Once confirmation is received, exchange the information to then finalize
    fn alice_communicate(
        &mut self,
        peer: PeerId,
        swap_id: SharedSwapId,
        local_swap_id: LocalSwapId,
        create_swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) {
        let addresses = self.announce.addresses_of_peer(&peer);
        self.secret_hash
            .register_addresses(peer.clone(), addresses.clone());
        self.ethereum_identity
            .register_addresses(peer.clone(), addresses.clone());
        self.lightning_identity
            .register_addresses(peer.clone(), addresses.clone());
        self.finalize.register_addresses(peer.clone(), addresses);

        self.ethereum_identity.send(
            peer.clone(),
            ethereum_identity::Message::new(swap_id, create_swap_params.ethereum_identity),
        );
        self.lightning_identity.send(
            peer.clone(),
            lightning_identity::Message::new(swap_id, create_swap_params.lightning_identity),
        );

        let seed = self.seed.derive_swap_seed(local_swap_id);
        let secret_hash = SecretHash::new(seed.derive_secret());

        self.secret_hashes.insert(swap_id, secret_hash);
        self.secret_hash
            .send(peer, secret_hash::Message::new(swap_id, secret_hash));

        self.communication_state
            .insert(swap_id, CommunicationState::default());
    }

    /// After announcement, confirm and then exchange information for the swap,
    /// once done, go finalize
    fn bob_communicate(
        &mut self,
        peer: libp2p::PeerId,
        io: ReplySubstream<NegotiatedSubstream>,
        shared_swap_id: SharedSwapId,
        create_swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) {
        // Confirm
        tokio::task::spawn(io.send(shared_swap_id));

        let addresses = self.announce.addresses_of_peer(&peer);
        self.secret_hash
            .register_addresses(peer.clone(), addresses.clone());
        self.ethereum_identity
            .register_addresses(peer.clone(), addresses.clone());
        self.lightning_identity
            .register_addresses(peer.clone(), addresses.clone());
        self.finalize.register_addresses(peer.clone(), addresses);

        // Communicate
        self.ethereum_identity.send(
            peer.clone(),
            ethereum_identity::Message::new(shared_swap_id, create_swap_params.ethereum_identity),
        );
        self.lightning_identity.send(
            peer,
            lightning_identity::Message::new(shared_swap_id, create_swap_params.lightning_identity),
        );

        self.communication_state
            .insert(shared_swap_id, CommunicationState::default());
    }

    fn poll<BIE>(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<BIE, BehaviourOutEvent>> {
        let time_limit = Timestamp::now().minus(PENDING_SWAP_EXPIRY_SECS);
        self.swaps.clean_up_pending_swaps(time_limit);

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        // We trust in libp2p to poll us.
        Poll::Pending
    }
}

#[derive(thiserror::Error, Clone, Copy, Debug)]
pub struct SwapExists;

impl fmt::Display for SwapExists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This impl is required to build but want to use a static string for
        // this when returning it via the REST API.
        write!(f, "")
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<secret_hash::Message>> for Comit {
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<secret_hash::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message:
                    secret_hash::Message {
                        swap_id,
                        secret_hash,
                    },
            } => {
                self.secret_hashes
                    .insert(swap_id, SecretHash::from(secret_hash));

                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("must exist");

                state.secret_hash_sent_or_received = true;

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message:
                    secret_hash::Message {
                        swap_id,
                        secret_hash,
                    },
            } => {
                self.secret_hashes
                    .insert(swap_id, SecretHash::from(secret_hash));

                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("should exist");

                state.secret_hash_sent_or_received = true;

                (peer, swap_id)
            }
        };

        let state = self.communication_state.get(&swap_id).unwrap();

        // check if we are done
        if self.ethereum_identities.contains_key(&swap_id)
            && self.lightning_identities.contains_key(&swap_id)
            && state.lightning_identity_sent
            && state.ethereum_identity_sent
            && state.secret_hash_sent_or_received
        {
            self.finalize.send(peer, finalize::Message::new(swap_id));
        }
    }
}

// It is already split in smaller functions
#[allow(clippy::cognitive_complexity)]
impl NetworkBehaviourEventProcess<announce::behaviour::BehaviourOutEvent> for Comit {
    fn inject_event(&mut self, event: announce::behaviour::BehaviourOutEvent) {
        match event {
            announce::behaviour::BehaviourOutEvent::ReceivedAnnouncement { peer, io } => {
                tracing::info!("Peer {} announced a swap ({})", peer, io.swap_digest);
                let span =
                    tracing::trace_span!("swap", digest = format_args!("{}", io.swap_digest));
                let _enter = span.enter();
                match self
                    .swaps
                    .move_pending_announcement_to_communicate(&io.swap_digest, &peer)
                {
                    Ok((shared_swap_id, create_params)) => {
                        tracing::debug!("Swap confirmation and communication has started.");
                        self.bob_communicate(peer, *io, shared_swap_id, create_params);
                    }
                    Err(swaps::Error::NotFound) => {
                        tracing::debug!("Swap has not been created yet, parking it.");
                        let _ = self
                            .swaps
                            .insert_pending_creation((&io.swap_digest).clone(), peer, *io)
                            .map_err(|_| {
                                tracing::error!(
                                    "Swap already known, Alice appeared to have sent it twice."
                                )
                            });
                    }
                    Err(err) => tracing::warn!(
                        "Announcement for {} was not processed due to {}",
                        io.swap_digest,
                        err
                    ),
                }
            }
            announce::behaviour::BehaviourOutEvent::ReceivedConfirmation {
                peer,
                swap_digest,
                swap_id: shared_swap_id,
            } => {
                let (local_swap_id, create_params) = self
                    .swaps
                    .move_pending_confirmation_to_communicate(&swap_digest, shared_swap_id)
                    .expect("we must know about this digest");

                self.alice_communicate(peer, shared_swap_id, local_swap_id, create_params);
            }
            announce::behaviour::BehaviourOutEvent::Error { peer, error } => {
                tracing::warn!(
                    "failed to complete announce protocol with {} because {:?}",
                    peer,
                    error
                );
            }
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<ethereum_identity::Message>>
    for Comit
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<ethereum_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: ethereum_identity::Message { swap_id, address },
            } => {
                self.ethereum_identities
                    .insert(swap_id, identity::Ethereum::from(address));

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: ethereum_identity::Message { swap_id, .. },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.ethereum_identity_sent = true;

                (peer, swap_id)
            }
        };

        let state = self.communication_state.get(&swap_id).unwrap();

        // check if we are done
        if self.ethereum_identities.contains_key(&swap_id)
            && self.lightning_identities.contains_key(&swap_id)
            && state.lightning_identity_sent
            && state.ethereum_identity_sent
            && state.secret_hash_sent_or_received
        {
            self.finalize.send(peer, finalize::Message::new(swap_id));
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<lightning_identity::Message>>
    for Comit
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<lightning_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: lightning_identity::Message { swap_id, pubkey },
            } => {
                self.lightning_identities.insert(
                    swap_id,
                    bitcoin::PublicKey::from_slice(&pubkey).unwrap().into(),
                );

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: lightning_identity::Message { swap_id, .. },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.lightning_identity_sent = true;

                (peer, swap_id)
            }
        };

        let state = self.communication_state.get(&swap_id).unwrap();

        // check if we are done
        if self.ethereum_identities.contains_key(&swap_id)
            && self.lightning_identities.contains_key(&swap_id)
            && state.lightning_identity_sent
            && state.ethereum_identity_sent
            && state.secret_hash_sent_or_received
        {
            self.finalize.send(peer, finalize::Message::new(swap_id));
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<finalize::Message>> for Comit {
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<finalize::Message>) {
        let (_, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: finalize::Message { swap_id },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.received_finalized = true;

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: finalize::Message { swap_id },
            } => {
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("this should exist");

                state.sent_finalized = true;

                (peer, swap_id)
            }
        };

        let state = self
            .communication_state
            .get_mut(&swap_id)
            .expect("this should exist");

        if state.sent_finalized && state.received_finalized {
            tracing::info!("Swap {} is finalized.", swap_id);
            let (local_swap_id, create_swap_params) = self
                .swaps
                .finalize_swap(&swap_id)
                .expect("Swap should be known");

            let secret_hash = self
                .secret_hashes
                .get(&swap_id)
                .copied()
                .expect("must exist");

            let ethereum_identity = self.ethereum_identities.get(&swap_id).copied().unwrap();
            let lightning_identity = self.lightning_identities.get(&swap_id).copied().unwrap();

            self.events.push_back(BehaviourOutEvent::SwapFinalized {
                local_swap_id,
                swap_params: create_swap_params,
                secret_hash,
                ethereum_identity,
                lightning_identity,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::{ethereum::FromWei, Erc20Quantity},
        lightning,
        network::{test_swarm, DialInformation},
    };
    use comit::{asset, RelativeTime};
    use digest::Digest;
    use futures::future;
    use libp2p::{multiaddr::Multiaddr, PeerId};
    use rand::thread_rng;

    fn make_alice_swap_params(
        bob_peer_id: PeerId,
        bob_addr: Multiaddr,
        erc20: asset::Erc20,
        lnbtc: asset::Bitcoin,
        ethereum_absolute_expiry: Timestamp,
        lightning_cltv_expiry: RelativeTime,
    ) -> Herc20HalightBitcoinCreateSwapParams {
        Herc20HalightBitcoinCreateSwapParams {
            role: Role::Alice,
            peer: DialInformation {
                peer_id: bob_peer_id,
                address_hint: Some(bob_addr),
            },
            ethereum_identity: identity::Ethereum::random(),
            ethereum_absolute_expiry,
            ethereum_amount: erc20.quantity,
            lightning_identity: lightning::PublicKey::random(),
            lightning_cltv_expiry,
            lightning_amount: lnbtc,
            token_contract: erc20.token_contract,
        }
    }

    fn make_bob_swap_params(
        alice_peer_id: PeerId,
        erc20: asset::Erc20,
        lnbtc: asset::Bitcoin,
        ethereum_absolute_expiry: Timestamp,
        lightning_cltv_expiry: RelativeTime,
    ) -> Herc20HalightBitcoinCreateSwapParams {
        Herc20HalightBitcoinCreateSwapParams {
            role: Role::Bob,
            peer: DialInformation {
                peer_id: alice_peer_id,
                address_hint: None,
            },
            ethereum_identity: identity::Ethereum::random(),
            ethereum_absolute_expiry,
            ethereum_amount: erc20.quantity,
            lightning_identity: lightning::PublicKey::random(),
            lightning_cltv_expiry,
            lightning_amount: lnbtc,
            token_contract: erc20.token_contract,
        }
    }

    #[tokio::test]
    async fn finalize_lightning_ethereum_swap_success() {
        // arrange
        let (mut alice_swarm, _, alice_peer_id) =
            test_swarm::new(Comit::new(RootSeed::new_random(thread_rng()).unwrap()));
        let (mut bob_swarm, bob_addr, bob_peer_id) =
            test_swarm::new(Comit::new(RootSeed::new_random(thread_rng()).unwrap()));

        let erc20 = asset::Erc20 {
            token_contract: Default::default(),
            quantity: Erc20Quantity::from_wei(9_001_000_000_000_000_000_000u128),
        };

        let lnbtc = asset::Bitcoin::from_sat(42);
        let ethereum_expiry = Timestamp::from(100);
        let lightning_expiry = RelativeTime::from(200);

        alice_swarm
            .initiate_communication(
                LocalSwapId::default(),
                make_alice_swap_params(
                    bob_peer_id,
                    bob_addr,
                    erc20.clone(),
                    lnbtc,
                    ethereum_expiry,
                    lightning_expiry,
                ),
            )
            .expect("initiate communication for alice");
        bob_swarm
            .initiate_communication(
                LocalSwapId::default(),
                make_bob_swap_params(
                    alice_peer_id,
                    erc20,
                    lnbtc,
                    ethereum_expiry,
                    lightning_expiry,
                ),
            )
            .expect("initiate communication for bob");

        // act
        let (alice_event, bob_event) = future::join(alice_swarm.next(), bob_swarm.next()).await;

        // assert
        match (alice_event, bob_event) {
            (
                BehaviourOutEvent::SwapFinalized {
                    swap_params: alice_swap_params,
                    ..
                },
                BehaviourOutEvent::SwapFinalized {
                    swap_params: bob_swap_params,
                    ..
                },
            ) => {
                assert_eq!(bob_swap_params.digest(), alice_swap_params.digest());
            }
        }
    }
}
