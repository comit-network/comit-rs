use crate::{
    asset, identity,
    network::{
        oneshot_behaviour,
        protocols::{
            announce,
            announce::{behaviour::Announce, SwapDigest},
            ethereum_identity, finalize, lightning_identity, secret_hash,
        },
    },
    seed::{DeriveSwapSeed, RootSeed},
    swap_protocols::{
        ledger::{ethereum::ChainId, lightning, Ethereum},
        rfc003::{create_swap::HtlcParams, DeriveSecret, Secret, SecretHash},
        HanEtherereumHalightBitcoinCreateSwapParams, LocalSwapId, Role, SharedSwapId,
    },
    timestamp::Timestamp,
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use digest::Digest;
use futures::AsyncWriteExt;
use libp2p::{
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    task::{Context, Poll},
};

/// Event emitted  by the `ComitLn` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    SwapFinalized {
        local_swap_id: LocalSwapId,
        swap_params: HanEtherereumHalightBitcoinCreateSwapParams,
        secret_hash: SecretHash,
        ethereum_identity: identity::Ethereum,
    },
}

#[derive(NetworkBehaviour, Debug)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct ComitLN {
    announce: Announce,
    secret_hash: oneshot_behaviour::Behaviour<secret_hash::Message>,
    ethereum_identity: oneshot_behaviour::Behaviour<ethereum_identity::Message>,
    lightning_identity: oneshot_behaviour::Behaviour<lightning_identity::Message>,
    finalize: oneshot_behaviour::Behaviour<finalize::Message>,

    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,

    #[behaviour(ignore)]
    swaps_waiting_for_announcement: HashMap<SwapDigest, LocalSwapId>,
    #[behaviour(ignore)]
    swaps_waiting_for_creation: Vec<SwapDigest>,
    #[behaviour(ignore)]
    swaps: HashMap<LocalSwapId, HanEtherereumHalightBitcoinCreateSwapParams>,
    #[behaviour(ignore)]
    swap_ids: HashMap<LocalSwapId, SharedSwapId>,
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

impl ComitLN {
    pub fn new(seed: RootSeed) -> Self {
        ComitLN {
            announce: Default::default(),
            secret_hash: Default::default(),
            ethereum_identity: Default::default(),
            lightning_identity: Default::default(),
            finalize: Default::default(),
            events: VecDeque::new(),
            swaps_waiting_for_announcement: Default::default(),
            swaps_waiting_for_creation: Default::default(),
            swaps: Default::default(),
            swap_ids: Default::default(),
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
        create_swap_params: HanEtherereumHalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        let digest = create_swap_params.clone().digest();

        if self.swaps_waiting_for_announcement.contains_key(&digest) {
            anyhow::bail!(SwapExists)
        }
        self.swaps.insert(id, create_swap_params.clone());
        self.swaps_waiting_for_announcement
            .insert(digest.clone(), id);

        match create_swap_params.role {
            Role::Alice => {
                self.announce
                    .start_announce_protocol(digest, create_swap_params.peer);
            }
            Role::Bob => {
                tracing::info!("Swap waiting for announcement: {}", digest);
            }
        }

        Ok(())
    }

    pub fn get_finalized_swap(&self, swap_id: LocalSwapId) -> Option<FinalizedSwap> {
        let create_swap_params = match self.swaps.get(&swap_id) {
            Some(body) => body,
            None => return None,
        };

        let secret = match create_swap_params.role {
            Role::Alice => Some(self.seed.derive_swap_seed(swap_id).derive_secret()),
            Role::Bob => None,
        };

        let id = match self.swap_ids.get(&swap_id).copied() {
            Some(id) => id,
            None => return None,
        };

        let alpha_ledger_redeem_identity = match create_swap_params.role {
            Role::Alice => match self.ethereum_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
            Role::Bob => create_swap_params.ethereum_identity.into(),
        };
        let alpha_ledger_refund_identity = match create_swap_params.role {
            Role::Alice => create_swap_params.ethereum_identity.into(),
            Role::Bob => match self.ethereum_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
        };
        let beta_ledger_redeem_identity = match create_swap_params.role {
            Role::Alice => create_swap_params.lightning_identity,
            Role::Bob => match self.lightning_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
        };
        let beta_ledger_refund_identity = match create_swap_params.role {
            Role::Alice => match self.lightning_identities.get(&id).copied() {
                Some(identity) => identity,
                None => return None,
            },
            Role::Bob => create_swap_params.lightning_identity,
        };

        Some(FinalizedSwap {
            alpha_ledger: Ethereum::new(ChainId::regtest()),
            beta_ledger: lightning::Regtest,
            alpha_asset: create_swap_params.ethereum_amount.clone(),
            beta_asset: create_swap_params.lightning_amount,
            alpha_ledger_redeem_identity,
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            beta_ledger_refund_identity,
            alpha_expiry: create_swap_params.ethereum_absolute_expiry,
            beta_expiry: create_swap_params.lightning_cltv_expiry,
            swap_id,
            secret,
            secret_hash: match self.secret_hashes.get(&id).copied() {
                Some(secret_hash) => secret_hash,
                None => return None,
            },
            role: create_swap_params.role,
        })
    }

    fn poll<BIE>(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<BIE, BehaviourOutEvent>> {
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

#[derive(Debug)]
pub struct FinalizedSwap {
    pub alpha_ledger: Ethereum,
    pub beta_ledger: lightning::Regtest,
    pub alpha_asset: asset::Ether,
    pub beta_asset: asset::Bitcoin,
    pub alpha_ledger_refund_identity: identity::Ethereum,
    pub alpha_ledger_redeem_identity: identity::Ethereum,
    pub beta_ledger_refund_identity: identity::Lightning,
    pub beta_ledger_redeem_identity: identity::Lightning,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub swap_id: LocalSwapId,
    pub secret_hash: SecretHash,
    pub secret: Option<Secret>,
    pub role: Role,
}

impl FinalizedSwap {
    pub fn han_params(&self) -> EtherHtlc {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: Ethereum::new(ChainId::regtest()),
            redeem_identity: self.alpha_ledger_redeem_identity,
            refund_identity: self.alpha_ledger_refund_identity,
            expiry: self.alpha_expiry,
            secret_hash: self.secret_hash,
        }
        .into()
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<secret_hash::Message>> for ComitLN {
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

impl NetworkBehaviourEventProcess<announce::behaviour::BehaviourOutEvent> for ComitLN {
    fn inject_event(&mut self, event: announce::behaviour::BehaviourOutEvent) {
        match event {
            announce::behaviour::BehaviourOutEvent::ReceivedAnnouncement { peer, mut io } => {
                tracing::info!("Peer {} announced a swap ({})", peer, io.swap_digest);
                // Check if there are any errors before modifying the hash-map.
                match self.swaps_waiting_for_announcement.get(&io.swap_digest) {
                    Some(local_swap_id) => {
                        // Verify that the peer-id announcing the swap matches the peer-id agreed on
                        // in the swap parameters. In case they don't match
                        // the swap has to stay available in swaps awaiting announcement
                        // but the current announcement will be rejected by closing the response
                        // channel.

                        let create_swap_params = self.swaps.get(&local_swap_id).unwrap();
                        if peer != create_swap_params.peer.peer_id {
                            tracing::error!(
                                "Peer {} announced a swap ({}), but the peer-id {} of the swap awaiting announcement does not match.",
                                peer,
                                io.swap_digest,
                                create_swap_params.peer.peer_id
                            );
                            tokio::task::spawn(async move {
                                let _ = io.io.close().await;
                            });

                            return;
                        }
                    }
                    None => {
                        self.swaps_waiting_for_creation.push(io.swap_digest);

                        return;
                    }
                }

                if let Some(local_swap_id) =
                    self.swaps_waiting_for_announcement.remove(&io.swap_digest)
                {
                    let create_swap_params = self.swaps.get(&local_swap_id).unwrap();
                    let shared_swap_id = SharedSwapId::default();
                    self.swap_ids
                        .insert(local_swap_id.clone(), shared_swap_id.clone());

                    tokio::task::spawn(io.send(shared_swap_id));

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
                        ethereum_identity::Message::new(
                            shared_swap_id,
                            create_swap_params.ethereum_identity.into(),
                        ),
                    );
                    self.lightning_identity.send(
                        peer,
                        lightning_identity::Message::new(
                            shared_swap_id,
                            create_swap_params.lightning_identity,
                        ),
                    );

                    self.communication_state
                        .insert(shared_swap_id, CommunicationState::default());
                }
            }
            announce::behaviour::BehaviourOutEvent::ReceivedConfirmation {
                peer,
                swap_digest,
                swap_id,
            } => {
                let local_swap_id = self
                    .swaps_waiting_for_announcement
                    .remove(&swap_digest)
                    .expect("we must know about this digest");

                self.swap_ids.insert(local_swap_id, swap_id);

                let addresses = self.announce.addresses_of_peer(&peer);
                self.secret_hash
                    .register_addresses(peer.clone(), addresses.clone());
                self.ethereum_identity
                    .register_addresses(peer.clone(), addresses.clone());
                self.lightning_identity
                    .register_addresses(peer.clone(), addresses.clone());
                self.finalize.register_addresses(peer.clone(), addresses);

                let create_swap_params = self.swaps.get(&local_swap_id).unwrap();

                self.ethereum_identity.send(
                    peer.clone(),
                    ethereum_identity::Message::new(
                        swap_id,
                        create_swap_params.ethereum_identity.into(),
                    ),
                );
                self.lightning_identity.send(
                    peer.clone(),
                    lightning_identity::Message::new(
                        swap_id,
                        create_swap_params.lightning_identity,
                    ),
                );

                let seed = self.seed.derive_swap_seed(local_swap_id);
                let secret_hash = seed.derive_secret().hash();

                self.secret_hashes.insert(swap_id, secret_hash);
                self.secret_hash
                    .send(peer, secret_hash::Message::new(swap_id, secret_hash));

                self.communication_state
                    .insert(swap_id, CommunicationState::default());
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
    for ComitLN
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
    for ComitLN
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

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<finalize::Message>> for ComitLN {
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
            let local_swap_id = self
                .swap_ids
                .iter()
                .find_map(
                    |(key, value)| {
                        if *value == swap_id {
                            Some(key)
                        } else {
                            None
                        }
                    },
                )
                .copied()
                .unwrap();

            let create_swap_params = self
                .swaps
                .get(&local_swap_id)
                .cloned()
                .expect("create swap params exist");

            let secret_hash = self
                .secret_hashes
                .get(&swap_id)
                .copied()
                .expect("must exist");

            let ethereum_identity = self.ethereum_identities.get(&swap_id).copied().unwrap();

            self.swaps_waiting_for_announcement
                .retain(|_, id| *id != local_swap_id);

            self.events.push_back(BehaviourOutEvent::SwapFinalized {
                local_swap_id,
                swap_params: create_swap_params,
                secret_hash,
                ethereum_identity,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::{ethereum::FromWei, Ether},
        lightning,
        network::{test_swarm, DialInformation},
        swap_protocols::EthereumIdentity,
    };
    use digest::Digest;
    use futures::future;
    use libp2p::{multiaddr::Multiaddr, PeerId};
    use rand::thread_rng;

    fn make_alice_swap_params(
        bob_peer_id: PeerId,
        bob_addr: Multiaddr,
        ether: asset::Ether,
        lnbtc: asset::Bitcoin,
        ethereum_absolute_expiry: Timestamp,
        lightning_cltv_expiry: Timestamp,
    ) -> HanEtherereumHalightBitcoinCreateSwapParams {
        HanEtherereumHalightBitcoinCreateSwapParams {
            role: Role::Alice,
            peer: DialInformation {
                peer_id: bob_peer_id,
                address_hint: Some(bob_addr),
            },
            ethereum_identity: EthereumIdentity::from(identity::Ethereum::random()),
            ethereum_absolute_expiry,
            ethereum_amount: ether,
            lightning_identity: lightning::PublicKey::random(),
            lightning_cltv_expiry,
            lightning_amount: lnbtc,
        }
    }

    fn make_bob_swap_params(
        alice_peer_id: PeerId,
        ether: asset::Ether,
        lnbtc: asset::Bitcoin,
        ethereum_absolute_expiry: Timestamp,
        lightning_cltv_expiry: Timestamp,
    ) -> HanEtherereumHalightBitcoinCreateSwapParams {
        HanEtherereumHalightBitcoinCreateSwapParams {
            role: Role::Bob,
            peer: DialInformation {
                peer_id: alice_peer_id,
                address_hint: None,
            },
            ethereum_identity: EthereumIdentity::from(identity::Ethereum::random()),
            ethereum_absolute_expiry,
            ethereum_amount: ether,
            lightning_identity: lightning::PublicKey::random(),
            lightning_cltv_expiry,
            lightning_amount: lnbtc,
        }
    }

    #[tokio::test]
    async fn finalize_lightning_ethereum_swap_success() {
        // arrange
        let (mut alice_swarm, _, alice_peer_id) =
            test_swarm::new(ComitLN::new(RootSeed::new_random(thread_rng()).unwrap()));
        let (mut bob_swarm, bob_addr, bob_peer_id) =
            test_swarm::new(ComitLN::new(RootSeed::new_random(thread_rng()).unwrap()));

        let ether = Ether::from_wei(9_001_000_000_000_000_000_000u128);
        let lnbtc = asset::Bitcoin::from_sat(42);
        let ethereum_expiry = Timestamp::from(100);
        let lightning_expiry = Timestamp::from(200);

        alice_swarm
            .initiate_communication(
                LocalSwapId::default(),
                make_alice_swap_params(
                    bob_peer_id,
                    bob_addr,
                    ether.clone(),
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
                    ether,
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
                    local_swap_id: _alice_local_swap_id,
                    swap_params: alice_swap_params,
                    secret_hash: _alice_secret_hash,
                    ethereum_identity: _alice_eth_id,
                },
                BehaviourOutEvent::SwapFinalized {
                    local_swap_id: _bob_local_swap_id,
                    swap_params: bob_swap_params,
                    secret_hash: _bob_secret_hash,
                    ethereum_identity: _bob_eth_id,
                },
            ) => {
                assert_eq!(bob_swap_params.digest(), alice_swap_params.digest());
            }
        }
    }
}
