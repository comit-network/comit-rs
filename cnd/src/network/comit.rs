use crate::{
    identity,
    network::Identities,
    seed::RootSeed,
    swap_protocols::{LocalSwapId, SharedSwapId},
    timestamp::Timestamp,
};
use comit::{
    network::{
        oneshot_behaviour,
        protocols::{
            announce,
            announce::{behaviour::Announce, protocol::ReplySubstream, SwapDigest},
            ethereum_identity, finalize, lightning_identity, secret_hash,
        },
        DialInformation,
    },
    SecretHash,
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

/// Event emitted by the `Comit` behaviour.
#[derive(Clone, Copy, Debug)]
pub struct BehaviourOutEvent {
    pub local_swap_id: LocalSwapId,
    pub remote_data: RemoteData,
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
    remote_data: HashMap<SharedSwapId, RemoteData>,
    #[behaviour(ignore)]
    communication_states: HashMap<SharedSwapId, CommunicationState>,
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
            remote_data: Default::default(),
            communication_states: Default::default(),
            seed,
        }
    }

    pub fn initiate_communication_for_alice(
        &mut self,
        local_swap_id: LocalSwapId,
        dial_info: DialInformation,
        digest: SwapDigest,
        data: LocalData,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting announcement for swap: {}", digest);
        self.announce
            .start_announce_protocol(digest.clone(), dial_info);
        self.swaps
            .create_as_pending_confirmation(digest, local_swap_id, data)?;

        Ok(())
    }

    pub fn initiate_communication_for_bob(
        &mut self,
        local_swap_id: LocalSwapId,
        dial_info: DialInformation,
        digest: SwapDigest,
        data: LocalData,
    ) -> anyhow::Result<()> {
        if let Ok((shared_swap_id, peer_id, io)) = self.swaps.move_pending_creation_to_communicate(
            &digest,
            local_swap_id,
            dial_info.peer_id.clone(),
            data.clone(),
        ) {
            tracing::info!("Confirm & communicate for swap: {}", digest);
            Self::confirm(shared_swap_id, io);
            self.communicate(shared_swap_id, peer_id, data)
        } else {
            self.swaps.create_as_pending_announcement(
                digest.clone(),
                local_swap_id,
                dial_info.peer_id,
                data,
            )?;
            tracing::debug!("Swap {} waiting for announcement", digest);
        }

        Ok(())
    }

    pub fn get_local_data(&self, swap_id: &LocalSwapId) -> Option<LocalData> {
        self.swaps.get_local_data(swap_id)
    }

    fn confirm(shared_swap_id: SharedSwapId, io: ReplySubstream<NegotiatedSubstream>) {
        tokio::task::spawn(io.send(shared_swap_id));
    }

    fn communicate(
        &mut self,
        shared_swap_id: SharedSwapId,
        peer_id: libp2p::PeerId,
        data: LocalData,
    ) {
        let addresses = self.announce.addresses_of_peer(&peer_id);
        self.secret_hash
            .register_addresses(peer_id.clone(), addresses.clone());
        self.ethereum_identity
            .register_addresses(peer_id.clone(), addresses.clone());
        self.lightning_identity
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
            let ethereum_sorted = if data.ethereum_identity.is_some() {
                remote_data.ethereum_identity.is_some() && state.ethereum_identity_sent
            } else {
                true
            };

            let lightning_sorted = if data.lightning_identity.is_some() {
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
                self.remote_data_insert(swap_id.clone(), SecretHash::from(secret_hash));

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
                if let Some(data) = self
                    .swaps
                    .move_pending_confirmation_to_communicate(&swap_digest, shared_swap_id)
                {
                    self.communicate(shared_swap_id, peer, data);
                } else {
                    tracing::warn!(
                        "Confirmation received for unknown swap {} from {}",
                        shared_swap_id,
                        peer
                    );
                }
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
                    Ok(identity) => self.remote_data_insert::<identity::Lightning>(
                        swap_id.clone(),
                        identity.into(),
                    ),
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
    // Currently we only support herc20/halight swap, as we add protocol
    // combinations the identities will be added here.
    pub secret_hash: Option<SecretHash>,
    pub shared_swap_id: Option<SharedSwapId>,
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
}

impl LocalData {
    pub fn for_alice(secret_hash: SecretHash, identities: Identities) -> Self {
        LocalData {
            secret_hash: Some(secret_hash),
            shared_swap_id: None,
            ethereum_identity: identities.ethereum_identity,
            lightning_identity: identities.lightning_identity,
        }
    }

    pub fn for_bob(shared_swap_id: SharedSwapId, identities: Identities) -> Self {
        LocalData {
            secret_hash: None,
            shared_swap_id: Some(shared_swap_id),
            ethereum_identity: identities.ethereum_identity,
            lightning_identity: identities.lightning_identity,
        }
    }
}

/// All possible data that can be received from the remote node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemoteData {
    // Currently we only support herc20/halight swap, as we add protocol
    // combinations the identities will be added here.
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
    pub secret_hash: Option<SecretHash>,
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
        asset::ethereum::FromWei,
        network::{test_swarm, DialInformation},
    };
    use comit::{asset, network::swap_digest::Herc20Halight};
    use digest::Digest;
    use futures::future;
    use rand::thread_rng;
    use std::str::FromStr;

    #[tokio::test]
    async fn finalize_lightning_ethereum_swap_success() {
        // arrange
        let (mut alice_swarm, _, alice_peer_id) =
            test_swarm::new(Comit::new(RootSeed::new_random(thread_rng()).unwrap()));
        let (mut bob_swarm, bob_addr, bob_peer_id) =
            test_swarm::new(Comit::new(RootSeed::new_random(thread_rng()).unwrap()));

        let secret_hash = SecretHash::from_str(
            "bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf\
             bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf",
        )
        .unwrap();

        let alice_local_data = LocalData {
            secret_hash: Some(secret_hash),
            shared_swap_id: None,
            ethereum_identity: Some(identity::Ethereum::random()),
            lightning_identity: Some(identity::Lightning::random()),
        };

        let bob_local_data = LocalData {
            secret_hash: None,
            shared_swap_id: None, // We don't test this here.
            ethereum_identity: Some(identity::Ethereum::random()),
            lightning_identity: Some(identity::Lightning::random()),
        };

        let digest = Herc20Halight {
            ethereum_absolute_expiry: 12345.into(),
            erc20_amount: asset::Erc20Quantity::from_wei(9_001_000_000_000_000_000_000u128),
            token_contract: identity::Ethereum::random(),
            lightning_cltv_expiry: 12345.into(),
            lightning_amount: asset::Bitcoin::from_sat(1_000_000_000),
        }
        .digest();

        let want_alice_to_learn_from_bob = RemoteData {
            ethereum_identity: bob_local_data.ethereum_identity,
            lightning_identity: bob_local_data.lightning_identity,
            // This is not exactly 'learned' but it is in the behaviour out event for both roles.
            secret_hash: alice_local_data.secret_hash,
        };

        let want_bob_to_learn_from_alice = RemoteData {
            ethereum_identity: alice_local_data.ethereum_identity,
            lightning_identity: alice_local_data.lightning_identity,
            secret_hash: alice_local_data.secret_hash,
        };

        alice_swarm
            .initiate_communication_for_alice(
                LocalSwapId::default(),
                DialInformation {
                    peer_id: bob_peer_id,
                    address_hint: Some(bob_addr),
                },
                digest.clone(),
                alice_local_data,
            )
            .expect("initiate communication for alice");

        bob_swarm
            .initiate_communication_for_bob(
                LocalSwapId::default(),
                DialInformation {
                    peer_id: alice_peer_id,
                    address_hint: None,
                },
                digest,
                bob_local_data,
            )
            .expect("initiate communication for bob");

        // act
        let (alice_event, bob_event) = future::join(alice_swarm.next(), bob_swarm.next()).await;

        // assert
        let alice_learned = alice_event.remote_data;
        let bob_learned = bob_event.remote_data;
        assert_eq!(bob_learned, want_bob_to_learn_from_alice);
        assert_eq!(alice_learned, want_alice_to_learn_from_bob);
    }
}
