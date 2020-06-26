use crate::{identity, network::*, LocalSwapId, SecretHash, SharedSwapId, Timestamp};
use libp2p::{
    swarm::{
        NegotiatedSubstream, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour, PeerId,
};
use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};
use swaps::Swaps;

/// Setting it at 5 minutes
const PENDING_SWAP_EXPIRY_SECS: u32 = 5 * 60;

pub mod swaps;

/// Event emitted by the `Comit` behaviour.
#[derive(Clone, Copy, Debug)]
pub struct BehaviourOutEvent {
    pub local_swap_id: LocalSwapId,
    pub remote_data: RemoteData,
}

/// A network behaviour for syncing execution parameters for an upcoming swap.
///
/// Execution parameters are data like the secret hash or certain identities
/// that are going to be used in contracts that facilitate an atomic swap.
/// For execution parameters to be exchanged, the nodes must have already agreed
/// to a `SharedSwapId`.
#[derive(NetworkBehaviour, Debug, Default)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct ExecutionParameters {
    secret_hash: oneshot_behaviour::Behaviour<secret_hash::Message>,
    ethereum_identity: oneshot_behaviour::Behaviour<ethereum_identity::Message>,
    lightning_identity: oneshot_behaviour::Behaviour<lightning_identity::Message>,
    bitcoin_identity: oneshot_behaviour::Behaviour<bitcoin_identity::Message>,
    finalize: oneshot_behaviour::Behaviour<finalize::Message>,

    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    pub swaps: Swaps,
    #[behaviour(ignore)]
    remote_data: HashMap<SharedSwapId, RemoteData>,
    #[behaviour(ignore)]
    communication_states: HashMap<SharedSwapId, CommunicationState>,
}

#[derive(Debug, Default)]
struct CommunicationState {
    ethereum_identity_sent: bool,
    lightning_identity_sent: bool,
    bitcoin_identity_sent: bool,
    received_finalized: bool,
    sent_finalized: bool,
    secret_hash_sent_or_received: bool,
}

impl ExecutionParameters {
    pub fn confirm(
        shared_swap_id: SharedSwapId,
        io: announce::ReplySubstream<NegotiatedSubstream>,
    ) {
        tokio::task::spawn(io.send(shared_swap_id));
    }

    pub fn communicate(
        &mut self,
        shared_swap_id: SharedSwapId,
        peer_id: libp2p::PeerId,
        data: LocalData,
        addresses: Vec<Multiaddr>,
    ) {
        self.secret_hash
            .register_addresses(peer_id.clone(), addresses.clone());
        self.ethereum_identity
            .register_addresses(peer_id.clone(), addresses.clone());
        self.lightning_identity
            .register_addresses(peer_id.clone(), addresses.clone());
        self.bitcoin_identity
            .register_addresses(peer_id.clone(), addresses.clone());
        self.finalize.register_addresses(peer_id.clone(), addresses);

        // Communicate
        if let Some(ethereum_identity) = data.ethereum_identity {
            self.ethereum_identity.send(
                peer_id.clone(),
                ethereum_identity::Message::new(shared_swap_id, ethereum_identity),
            );
        }

        if let Some(lightning_identity) = data.lightning_identity {
            self.lightning_identity.send(
                peer_id.clone(),
                lightning_identity::Message::new(shared_swap_id, lightning_identity),
            );
        }

        if let Some(bitcoin_identity) = data.bitcoin_identity {
            self.bitcoin_identity.send(
                peer_id.clone(),
                bitcoin_identity::Message::new(shared_swap_id, bitcoin_identity),
            );
        }

        if let Some(secret_hash) = data.secret_hash {
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

        self.communication_states
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
        let local_swap_id = match self.swaps.get_local_swap_id(shared_swap_id) {
            Some(id) => id,
            None => return,
        };

        let state = self.communication_states.get(&shared_swap_id);
        let data = self.swaps.get_local_data(&local_swap_id);
        let remote_data = self.remote_data.get(&shared_swap_id);

        if let (Some(state), Some(data), Some(remote_data)) = (state, data, remote_data) {
            if data.ethereum_identity.is_some()
                && (remote_data.ethereum_identity.is_none() || !state.ethereum_identity_sent)
            {
                // Swap not yet finalized, Ethereum identities not synced.
                return;
            }

            if data.lightning_identity.is_some()
                && (remote_data.lightning_identity.is_none() || !state.lightning_identity_sent)
            {
                // Swap not yet finalized, Lightning identities not synced.
                return;
            }

            if data.bitcoin_identity.is_some()
                && (remote_data.bitcoin_identity.is_none() || !state.bitcoin_identity_sent)
            {
                // Swap not yet finalized, Bitcoin identities not synced.
                return;
            }

            if !state.secret_hash_sent_or_received {
                // Swap not yet finalized, secret hash not synced.
                return;
            }

            self.finalize
                .send(peer, finalize::Message::new(shared_swap_id));
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<secret_hash::Message>>
    for ExecutionParameters
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<secret_hash::Message>) {
        let option = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message:
                    secret_hash::Message {
                        swap_id,
                        secret_hash,
                    },
            } => {
                self.remote_data_insert(swap_id, SecretHash::from(secret_hash));

                match self.communication_states.get_mut(&swap_id) {
                    Some(state) => {
                        state.secret_hash_sent_or_received = true;
                        Some((peer, swap_id))
                    }
                    None => {
                        tracing::warn!(
                            "Secret hash received for unknown swap {} from {}",
                            swap_id,
                            peer
                        );
                        None
                    }
                }
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message:
                    secret_hash::Message {
                        swap_id,
                        secret_hash: _,
                    },
            } => {
                let state = self
                    .communication_states
                    .get_mut(&swap_id)
                    .expect("Swap should be known as we sent a message about it");

                state.secret_hash_sent_or_received = true;

                Some((peer, swap_id))
            }
        };

        if let Some((peer, swap_id)) = option {
            self.finalize(peer, swap_id)
        }
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<ethereum_identity::Message>>
    for ExecutionParameters
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
                    .communication_states
                    .get_mut(&swap_id)
                    .expect("Swap should be known as we sent a message about it");

                state.ethereum_identity_sent = true;

                (peer, swap_id)
            }
        };

        self.finalize(peer, swap_id)
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<lightning_identity::Message>>
    for ExecutionParameters
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<lightning_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: lightning_identity::Message { swap_id, pubkey },
            } => {
                match bitcoin::PublicKey::from_slice(&pubkey) {
                    Ok(identity) => {
                        self.remote_data_insert::<identity::Lightning>(swap_id, identity.into())
                    }
                    Err(_) => {
                        tracing::warn!("received an invalid lightning identity from counterparty");
                        return;
                    }
                };

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: lightning_identity::Message { swap_id, .. },
            } => {
                let state = self
                    .communication_states
                    .get_mut(&swap_id)
                    .expect("Swap should be known as we sent a message about it");

                state.lightning_identity_sent = true;

                (peer, swap_id)
            }
        };

        self.finalize(peer, swap_id)
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<bitcoin_identity::Message>>
    for ExecutionParameters
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<bitcoin_identity::Message>) {
        let (peer, swap_id) = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: bitcoin_identity::Message { swap_id, pubkey },
            } => {
                match bitcoin::PublicKey::from_slice(&pubkey) {
                    Ok(identity) => {
                        self.remote_data_insert::<identity::Bitcoin>(swap_id, identity.into())
                    }
                    Err(_) => {
                        tracing::warn!("received an invalid bitcoin identity from counterparty");
                        return;
                    }
                };

                (peer, swap_id)
            }
            oneshot_behaviour::OutEvent::Sent {
                peer,
                message: bitcoin_identity::Message { swap_id, .. },
            } => {
                let state = self
                    .communication_states
                    .get_mut(&swap_id)
                    .expect("Swap should be known as we sent a message about it");

                state.bitcoin_identity_sent = true;

                (peer, swap_id)
            }
        };

        self.finalize(peer, swap_id)
    }
}

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<finalize::Message>>
    for ExecutionParameters
{
    fn inject_event(&mut self, event: oneshot_behaviour::OutEvent<finalize::Message>) {
        let swap_id = match event {
            oneshot_behaviour::OutEvent::Received {
                peer,
                message: finalize::Message { swap_id },
            } => {
                if let Some(state) = self.communication_states.get_mut(&swap_id) {
                    state.received_finalized = true;

                    Some(swap_id)
                } else {
                    tracing::warn!(
                        "finalize message received for unknown swap {} from {}",
                        swap_id,
                        peer
                    );
                    None
                }
            }
            oneshot_behaviour::OutEvent::Sent {
                peer: _,
                message: finalize::Message { swap_id },
            } => {
                let state = self
                    .communication_states
                    .get_mut(&swap_id)
                    .expect("Swap should be known as we sent a message about it");

                state.sent_finalized = true;

                Some(swap_id)
            }
        };

        if let Some(swap_id) = swap_id {
            if let Some(state) = self.communication_states.get_mut(&swap_id) {
                if state.sent_finalized && state.received_finalized {
                    tracing::info!("Swap {} is finalized.", swap_id);
                    if let Ok(local_swap_id) = self.swaps.finalize_swap(&swap_id) {
                        if let Some(remote_data) = self.remote_data.get(&swap_id).cloned() {
                            self.events.push_back(BehaviourOutEvent {
                                local_swap_id,
                                remote_data,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// All possible data to be sent to the remote node for any protocol
/// combination.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalData {
    pub secret_hash: Option<SecretHash>,      // Known by Alice.
    pub shared_swap_id: Option<SharedSwapId>, // Known by Bob.
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
    pub bitcoin_identity: Option<identity::Bitcoin>,
}

impl LocalData {
    pub fn for_alice(secret_hash: SecretHash, identities: Identities) -> Self {
        LocalData {
            secret_hash: Some(secret_hash),
            shared_swap_id: None,
            ethereum_identity: identities.ethereum_identity,
            lightning_identity: identities.lightning_identity,
            bitcoin_identity: identities.bitcoin_identity,
        }
    }

    pub fn for_bob(shared_swap_id: SharedSwapId, identities: Identities) -> Self {
        LocalData {
            secret_hash: None,
            shared_swap_id: Some(shared_swap_id),
            ethereum_identity: identities.ethereum_identity,
            lightning_identity: identities.lightning_identity,
            bitcoin_identity: identities.bitcoin_identity,
        }
    }
}

/// All possible data that can be received from the remote node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemoteData {
    pub secret_hash: Option<SecretHash>, // Received by Bob from Alice.
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
    pub bitcoin_identity: Option<identity::Bitcoin>,
}

impl Default for RemoteData {
    fn default() -> Self {
        RemoteData {
            ethereum_identity: None,
            lightning_identity: None,
            bitcoin_identity: None,
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

impl Set<identity::Bitcoin> for RemoteData {
    fn set(&mut self, value: identity::Bitcoin) {
        self.bitcoin_identity = Some(value);
    }
}

impl Set<SecretHash> for RemoteData {
    fn set(&mut self, value: SecretHash) {
        self.secret_hash = Some(value);
    }
}
