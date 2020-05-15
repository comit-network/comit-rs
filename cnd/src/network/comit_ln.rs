use crate::{
    asset, identity,
    network::{
        oneshot_behaviour,
        protocols::{
            announce,
            announce::{behaviour::Announce, protocol::ReplySubstream, SwapDigest},
            ethereum_identity, finalize, lightning_identity, secret_hash,
        },
        DialInformation,
    },
    seed::RootSeed,
    swap_protocols::{
        ledger::{ethereum::ChainId, lightning, Ethereum},
        rfc003::{create_swap::HtlcParams, Secret, SecretHash},
        LocalSwapId, Role, SharedSwapId,
    },
    timestamp::{RelativeTime, Timestamp},
};
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
        data: Data,
        remote_data: RemoteData,
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
    swaps: Swaps<ReplySubstream<NegotiatedSubstream>>,
    #[behaviour(ignore)]
    remote_data: HashMap<SharedSwapId, RemoteData>,
    #[behaviour(ignore)]
    communication_state: HashMap<SharedSwapId, CommunicationState>,

    #[behaviour(ignore)]
    pub seed: RootSeed,
}

// TODO: This could be replaced with a function on remote_data/data
#[derive(Debug, Default)]
struct CommunicationState {
    ethereum_identity_sent: bool,
    lightning_identity_sent: bool,
    received_finalized: bool,
    sent_finalized: bool,
    secret_hash_sent_or_received: bool,
}

// TODO: Can probably now be "Comit"
impl ComitLN {
    pub fn new(seed: RootSeed) -> Self {
        ComitLN {
            announce: Default::default(),
            secret_hash: Default::default(),
            ethereum_identity: Default::default(),
            lightning_identity: Default::default(),
            finalize: Default::default(),
            events: VecDeque::new(),
            swaps: Default::default(),
            remote_data: Default::default(),
            communication_state: Default::default(),
            seed,
        }
    }

    pub fn initiate_communication(
        &mut self,
        local_swap_id: LocalSwapId,
        dial_info: DialInformation,
        role: Role, // TODO: This can be deduced by the presence of shared_local_id
        digest: SwapDigest,
        data: Data,
    ) -> anyhow::Result<()> {
        tracing::trace!("Swap creation request received: {}", digest);

        match role {
            Role::Alice => {
                tracing::info!("Starting announcement for swap: {}", digest);
                self.announce
                    .start_announce_protocol(digest.clone(), dial_info.clone());
                self.swaps
                    .create_as_pending_confirmation(digest, local_swap_id, data)?;
            }
            Role::Bob => {
                if let Ok((shared_swap_id, peer_id, io)) =
                    self.swaps.move_pending_creation_to_communicate(
                        &digest,
                        local_swap_id,
                        dial_info.peer_id,
                        data.clone(),
                    )
                {
                    tracing::info!("Confirm & communicate for swap: {}", digest);
                    Self::confirm(shared_swap_id, io);
                    self.communicate(shared_swap_id, peer_id, data)
                } else {
                    self.swaps.create_as_pending_announcement(
                        digest.clone(),
                        local_swap_id,
                        data,
                    )?;
                    tracing::debug!("Swap {} waiting for announcement", digest);
                }
            }
        }

        Ok(())
    }

    pub fn get_created_swap(&self, swap_id: &LocalSwapId) -> Option<Data> {
        self.swaps.get_created_swap(swap_id)
    }

    pub fn get_finalized_swap(&self, swap_id: LocalSwapId) -> Option<FinalizedSwap> {
        // TODO: This is strange, why do we do the "my identity" to "alpha/beta"
        // identity conversion here? This module is about communication and
        // should not need to know alpha/beta if it is not needed to communicate.

        unimplemented!()
        // let (id, data) = match self.swaps.get_announced_swap(&swap_id) {
        //     Some(swap) => swap,
        //     None => return None,
        // };
        //
        // let alpha_ledger_redeem_identity = match data.local_ethereum_identity
        // {     None => match
        // self.ethereum_identities.get(&id).copied() {
        //         Some(identity) => identity,
        //         None => return None,
        //     },
        //     Some(ethereum_identity) => ethereum_identity.into(),
        // };
        //
        // let alpha_ledger_refund_identity = match data.role {
        //     Role::Alice => data.ethereum_identity.into(),
        //     Role::Bob => match self.ethereum_identities.get(&id).copied() {
        //         Some(identity) => identity,
        //         None => return None,
        //     },
        // };
        // let beta_ledger_redeem_identity = match data.role {
        //     Role::Alice => data.lightning_identity,
        //     Role::Bob => match self.lightning_identities.get(&id).copied() {
        //         Some(identity) => identity,
        //         None => return None,
        //     },
        // };
        // let beta_ledger_refund_identity = match data.role {
        //     Role::Alice => match self.lightning_identities.get(&id).copied()
        // {         Some(identity) => identity,
        //         None => return None,
        //     },
        //     Role::Bob => data.lightning_identity,
        // };
        //
        // let erc20 = asset::Erc20 {
        //     token_contract: data.token_contract.into(),
        //     quantity: data.ethereum_amount,
        // };
        // Some(FinalizedSwap {
        //     alpha_ledger: Ethereum::new(ChainId::regtest()),
        //     beta_ledger: lightning::Regtest,
        //     alpha_asset: erc20,
        //     beta_asset: data.lightning_amount,
        //     alpha_ledger_redeem_identity,
        //     alpha_ledger_refund_identity,
        //     beta_ledger_redeem_identity,
        //     beta_ledger_refund_identity,
        //     alpha_expiry: data.ethereum_absolute_expiry,
        //     beta_expiry: data.lightning_cltv_expiry,
        //     swap_id,
        //     data.secret,
        //     secret_hash: match self.secret_hashes.get(&id).copied() {
        //         Some(secret_hash) => secret_hash,
        //         None => return None,
        //     },
        //     role: data.role,
        // })
    }

    fn confirm(shared_swap_id: SharedSwapId, io: ReplySubstream<NegotiatedSubstream>) {
        tokio::task::spawn(io.send(shared_swap_id));
    }

    fn communicate(&mut self, shared_swap_id: SharedSwapId, peer_id: libp2p::PeerId, data: Data) {
        let addresses = self.announce.addresses_of_peer(&peer_id);
        self.secret_hash
            .register_addresses(peer_id.clone(), addresses.clone());
        self.ethereum_identity
            .register_addresses(peer_id.clone(), addresses.clone());
        self.lightning_identity
            .register_addresses(peer_id.clone(), addresses.clone());
        self.finalize.register_addresses(peer_id.clone(), addresses);

        // Communicate
        if let Some(ethereum_identity) = data.local_ethereum_identity {
            self.ethereum_identity.send(
                peer_id.clone(),
                ethereum_identity::Message::new(shared_swap_id, ethereum_identity.into()),
            );
        }

        if let Some(lightning_identity) = data.local_lightning_identity {
            self.lightning_identity.send(
                peer_id.clone(),
                lightning_identity::Message::new(shared_swap_id, lightning_identity.into()),
            );
        }

        if let Some(secret) = data.secret {
            let secret_hash = secret.hash();
            // TODO: Create helper function or prettier way to do this
            let mut remote_data = self
                .remote_data
                .get(&shared_swap_id)
                .cloned()
                .unwrap_or_default();
            remote_data.secret_hash = Some(secret_hash);
            self.remote_data.insert(shared_swap_id, remote_data);

            self.secret_hash.send(
                peer_id,
                secret_hash::Message::new(shared_swap_id, secret_hash),
            );
        }

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

    fn remote_data_insert<T>(&mut self, shared_swap_id: SharedSwapId, value: T)
    where
        RemoteData: Set<T>,
    {
        let mut remote_data = self
            .remote_data
            .get(&shared_swap_id)
            .cloned()
            .unwrap_or_default();
        remote_data.set(value);
        self.remote_data.insert(shared_swap_id, remote_data);
    }

    fn finalize(&mut self, peer: PeerId, shared_swap_id: SharedSwapId) {
        let state = self.communication_state.get(&shared_swap_id);
        let data = self.swaps.get_from_shared_id(&shared_swap_id);
        let remote_data = self.remote_data.get(&shared_swap_id);

        if let (Some(state), Some(data), Some(remote_data)) = (state, data, remote_data) {
            let ethereum_sorted = if data.local_ethereum_identity.is_some() {
                remote_data.ethereum_identity.is_some() && state.ethereum_identity_sent
            } else {
                true
            };

            let lightning_sorted = if data.local_lightning_identity.is_some() {
                remote_data.lightning_identity.is_some() && state.lightning_identity_sent
            } else {
                true
            };

            if ethereum_sorted && lightning_sorted && state.secret_hash_sent_or_received {
                self.finalize
                    .send(peer, finalize::Message::new(shared_swap_id));
            }
        }
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

#[derive(Clone, Debug)]
pub struct FinalizedSwap {
    pub alpha_ledger: Ethereum,
    pub beta_ledger: lightning::Regtest,
    pub alpha_asset: asset::Erc20,
    pub beta_asset: asset::Bitcoin,
    pub alpha_ledger_refund_identity: identity::Ethereum,
    pub alpha_ledger_redeem_identity: identity::Ethereum,
    pub beta_ledger_refund_identity: identity::Lightning,
    pub beta_ledger_redeem_identity: identity::Lightning,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: RelativeTime,
    pub swap_id: LocalSwapId,
    pub secret_hash: SecretHash,
    pub secret: Option<Secret>,
    pub role: Role,
}

impl FinalizedSwap {
    pub fn herc20_params(&self) -> HtlcParams<Ethereum, asset::Erc20, identity::Ethereum> {
        HtlcParams {
            asset: self.alpha_asset.clone(),
            ledger: Ethereum::new(ChainId::regtest()),
            redeem_identity: self.alpha_ledger_redeem_identity,
            refund_identity: self.alpha_ledger_refund_identity,
            expiry: self.alpha_expiry,
            secret_hash: self.secret_hash,
        }
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
                self.remote_data_insert(swap_id.clone(), SecretHash::from(secret_hash));

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
                let state = self
                    .communication_state
                    .get_mut(&swap_id)
                    .expect("should exist");

                state.secret_hash_sent_or_received = true;

                (peer, swap_id)
            }
        };

        self.finalize(peer, swap_id)
    }
}

// It is already split in smaller functions
#[allow(clippy::cognitive_complexity)]
impl NetworkBehaviourEventProcess<announce::behaviour::BehaviourOutEvent> for ComitLN {
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
                        Self::confirm(shared_swap_id, *io);
                        self.communicate(shared_swap_id, peer, create_params);
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
                let (local_swap_id, data) = self
                    .swaps
                    .move_pending_confirmation_to_communicate(&swap_digest, shared_swap_id)
                    .expect("we must know about this digest");

                self.communicate(shared_swap_id, peer, data);
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

// TODO: This kind of implementations could go in their own sub module to make
// it clearer
impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<ethereum_identity::Message>>
    for ComitLN
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<ethereum_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: ethereum_identity::Message { swap_id, address },
            } => {
                self.remote_data_insert(swap_id, identity::Ethereum::from(address));

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

        self.finalize(peer, swap_id)
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
                // TODO: Remove this expect
                self.remote_data_insert(
                    swap_id.clone(),
                    bitcoin::PublicKey::from_slice(&pubkey)
                        .expect("We hope that secp likes the key the other party sent us")
                        .into(),
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

        self.finalize(peer, swap_id)
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
            tracing::info!("Swap {} is finalized.", swap_id);
            let (local_swap_id, data) = self
                .swaps
                .finalize_swap(&swap_id)
                .expect("Swap should be known");

            let remote_data = self.remote_data.get(&swap_id).cloned().unwrap();

            self.events.push_back(BehaviourOutEvent::SwapFinalized {
                local_swap_id,
                data,
                remote_data,
            });
        }
    }
}

/// All possible data to be sent to the remote node
#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub peer_id: PeerId,
    pub secret: Option<Secret>,
    pub shared_swap_id: Option<SharedSwapId>,
    pub local_ethereum_identity: Option<identity::Ethereum>,
    pub local_lightning_identity: Option<identity::Lightning>,
}

// TODO: Rename to LocalData
impl Data {
    pub fn new(
        peer_id: PeerId,
        secret: Option<Secret>,
        shared_swap_id: Option<SharedSwapId>,
        local_ethereum_identity: Option<identity::Ethereum>,
        local_lightning_identity: Option<identity::Lightning>,
    ) -> Self {
        Data {
            peer_id,
            secret,
            shared_swap_id,
            local_ethereum_identity,
            local_lightning_identity,
        }
    }
}

/// All possible data to be received from the remote node
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteData {
    ethereum_identity: Option<identity::Ethereum>,
    lightning_identity: Option<identity::Lightning>,
    secret_hash: Option<SecretHash>,
}

impl Default for RemoteData {
    fn default() -> Self {
        RemoteData {
            ethereum_identity: None,
            lightning_identity: None,
            secret_hash: None,
        }
    }
}

trait Set<T> {
    fn set(&mut self, value: T);
}

impl Set<identity::Ethereum> for RemoteData {
    fn set(&mut self, value: identity::Ethereum) {
        self.ethereum_identity = Some(value);
    }
}

impl Set<identity::Lightning> for RemoteData {
    fn set(&mut self, value: identity::Lightning) {
        self.lightning_identity = Some(value);
    }
}

impl Set<SecretHash> for RemoteData {
    fn set(&mut self, value: SecretHash) {
        self.secret_hash = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset::{ethereum::FromWei, Erc20Quantity},
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
            ethereum_identity: EthereumIdentity::from(identity::Ethereum::random()),
            ethereum_absolute_expiry,
            ethereum_amount: erc20.quantity,
            lightning_identity: lightning::PublicKey::random(),
            lightning_cltv_expiry,
            lightning_amount: lnbtc,
            token_contract: erc20.token_contract.into(),
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
            ethereum_identity: EthereumIdentity::from(identity::Ethereum::random()),
            ethereum_absolute_expiry,
            ethereum_amount: erc20.quantity,
            lightning_identity: lightning::PublicKey::random(),
            lightning_cltv_expiry,
            lightning_amount: lnbtc,
            token_contract: erc20.token_contract.into(),
        }
    }

    #[tokio::test]
    async fn finalize_lightning_ethereum_swap_success() {
        // arrange
        let (mut alice_swarm, _, alice_peer_id) =
            test_swarm::new(ComitLN::new(RootSeed::new_random(thread_rng()).unwrap()));
        let (mut bob_swarm, bob_addr, bob_peer_id) =
            test_swarm::new(ComitLN::new(RootSeed::new_random(thread_rng()).unwrap()));

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
