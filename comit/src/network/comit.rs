use crate::{identity, network::*, SecretHash};
use libp2p::{
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};

/// Event emitted by the `Comit` behaviour.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BehaviourOutEvent {
    SwapFinalized {
        shared_swap_id: SharedSwapId,
        remote_data: RemoteData,
    },
}

#[derive(NetworkBehaviour, Debug)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct Comit {
    secret_hash: oneshot_behaviour::Behaviour<secret_hash::Message>,
    ethereum_identity: oneshot_behaviour::Behaviour<ethereum_identity::Message>,
    lightning_identity: oneshot_behaviour::Behaviour<lightning_identity::Message>,
    bitcoin_identity: oneshot_behaviour::Behaviour<bitcoin_identity::Message>,
    finalize: oneshot_behaviour::Behaviour<finalize::Message>,

    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    remote_data: HashMap<SharedSwapId, RemoteData>,
    #[behaviour(ignore)]
    local_data: HashMap<SharedSwapId, LocalData>,
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

impl Default for Comit {
    fn default() -> Self {
        Comit {
            secret_hash: Default::default(),
            ethereum_identity: Default::default(),
            lightning_identity: Default::default(),
            bitcoin_identity: Default::default(),
            finalize: Default::default(),
            events: Default::default(),
            remote_data: Default::default(),
            local_data: Default::default(),
            communication_states: Default::default(),
        }
    }
}

impl Comit {
    pub fn communicate(
        &mut self,
        peer_id: libp2p::PeerId,
        shared_swap_id: SharedSwapId,
        data: LocalData,
    ) {
        self.local_data.insert(shared_swap_id, data);

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
        let data = self.local_data.get(&shared_swap_id);
        let state = self.communication_states.get(&shared_swap_id);
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

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<secret_hash::Message>> for Comit {
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
    for Comit
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
    for Comit
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
    for Comit
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

impl NetworkBehaviourEventProcess<oneshot_behaviour::OutEvent<finalize::Message>> for Comit {
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

        if let Some(shared_swap_id) = swap_id {
            if let Some(state) = self.communication_states.get_mut(&shared_swap_id) {
                if state.sent_finalized && state.received_finalized {
                    tracing::info!("Swap {} is finalized.", shared_swap_id);
                    if let Some(remote_data) = self.remote_data.get(&shared_swap_id).cloned() {
                        self.events.push_back(BehaviourOutEvent::SwapFinalized {
                            shared_swap_id,
                            remote_data,
                        });
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
    pub secret_hash: Option<SecretHash>, // Known by Alice.
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
    pub bitcoin_identity: Option<identity::Bitcoin>,
}

impl LocalData {
    pub fn for_alice(secret_hash: SecretHash, identities: Identities) -> Self {
        LocalData {
            secret_hash: Some(secret_hash),
            ethereum_identity: identities.ethereum_identity,
            lightning_identity: identities.lightning_identity,
            bitcoin_identity: identities.bitcoin_identity,
        }
    }

    pub fn for_bob(identities: Identities) -> Self {
        LocalData {
            secret_hash: None,
            ethereum_identity: identities.ethereum_identity,
            lightning_identity: identities.lightning_identity,
            bitcoin_identity: identities.bitcoin_identity,
        }
    }
}

/// All possible data that can be received from the remote node.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RemoteData {
    pub secret_hash: Option<SecretHash>, // Received by Bob from Alice.
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
    pub bitcoin_identity: Option<identity::Bitcoin>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::test::{await_events_or_timeout, connect, new_swarm};
    use std::str::FromStr;

    #[tokio::test]
    async fn finalize_lightning_ethereum_swap_success() {
        // arrange
        let (mut alice_swarm, _, alice_peer_id) = new_swarm(|_, _| Comit::default());
        let (mut bob_swarm, _, bob_peer_id) = new_swarm(|_, _| Comit::default());
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let secret_hash = SecretHash::from_str(
            "bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf\
             bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf",
        )
        .unwrap();
        let alice_local_data = LocalData {
            secret_hash: Some(secret_hash),
            ethereum_identity: Some(identity::Ethereum::random()),
            lightning_identity: Some(identity::Lightning::random()),
            bitcoin_identity: None,
        };
        let bob_local_data = LocalData {
            secret_hash: None,
            ethereum_identity: Some(identity::Ethereum::random()),
            lightning_identity: Some(identity::Lightning::random()),
            bitcoin_identity: None,
        };
        let shared_swap_id = SharedSwapId::default();

        // act
        alice_swarm.communicate(bob_peer_id, shared_swap_id, alice_local_data);
        bob_swarm.communicate(alice_peer_id, shared_swap_id, bob_local_data);
        let (alice_event, bob_event) =
            await_events_or_timeout(alice_swarm.next(), bob_swarm.next()).await;

        // assert
        let (what_alice_learned_from_bob, what_bob_learned_from_alice) =
            match (alice_event, bob_event) {
                (
                    BehaviourOutEvent::SwapFinalized {
                        remote_data: alice_remote_data,
                        ..
                    },
                    BehaviourOutEvent::SwapFinalized {
                        remote_data: bob_remote_data,
                        ..
                    },
                ) => (alice_remote_data, bob_remote_data),
            };
        assert_eq!(what_alice_learned_from_bob, RemoteData {
            // This is not exactly 'learned' but it is in the behaviour out event for both roles.
            secret_hash: alice_local_data.secret_hash,
            ethereum_identity: bob_local_data.ethereum_identity,
            lightning_identity: bob_local_data.lightning_identity,
            bitcoin_identity: None,
        });
        assert_eq!(what_bob_learned_from_alice, RemoteData {
            secret_hash: alice_local_data.secret_hash,
            ethereum_identity: alice_local_data.ethereum_identity,
            lightning_identity: alice_local_data.lightning_identity,
            bitcoin_identity: None,
        });
    }
}
