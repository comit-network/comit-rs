use crate::{
    network::{
        announce::{
            handler,
            handler::{Handler, HandlerEvent},
            protocol::OutboundConfig,
        },
        protocols::announce::protocol::Confirmed,
        SwapDigest,
    },
    DialInformation, SharedSwapId,
};
use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, PollParameters, ProtocolsHandler,
    },
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    task::{Context, Poll},
    time::Duration,
};

/// Network behaviour that implements the "announce" protocol.
///
/// The announce protocol allows two nodes to confirm the expectations about an
/// upcoming swap. In fact, Bob needs to confirm the swap through the announce
/// protocol for any further action to happen. Bob's confirmed contains the
/// `swap_id` which allows both parties to continue with the execution parameter
/// exchange protocols.
///
/// To confirm the expectations about a swap, both nodes start the announce
/// protocol with a `SwapDigest`. A `SwapDigest` is a fingerprint of all the
/// data that is relevant to a given swap. If both nodes compute the same
/// `SwapDigest` from the data they received from their users, all critical
/// parameters of the swap match and the swap execution can safely start.
#[derive(Debug)]
pub struct Announce {
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<OutboundConfig, BehaviourOutEvent>>,
    /// Stores connection state for nodes we connect to.
    connections: HashMap<PeerId, ConnectionState>,

    awaiting_announcements: HashSet<SwapDigest>,

    /// For how long Bob will buffer an incoming announcement before it expires.
    incoming_announcement_buffer_expiry: Duration,
}

impl Default for Announce {
    fn default() -> Self {
        let five_minutes = Duration::from_secs(5 * 60);

        Self::new(five_minutes)
    }
}

impl Announce {
    pub fn new(incoming_announcement_buffer_expiry: Duration) -> Self {
        Self {
            events: VecDeque::default(),
            connections: HashMap::default(),
            awaiting_announcements: HashSet::default(),
            incoming_announcement_buffer_expiry,
        }
    }

    /// Alice to announce a swap to Bob.
    ///
    /// This starts the announce protocol from Alice's perspective. In other
    /// words, Alice is going to send an announce message to Bob and wait
    /// for his confirmation.
    pub fn announce_swap(&mut self, swap_to_announce: SwapDigest, peer: DialInformation) {
        tracing::info!("Announcing swap {} to {}", swap_to_announce, peer.peer_id);

        match self.connections.entry(peer.peer_id.clone()) {
            Entry::Vacant(entry) => {
                self.events.push_back(NetworkBehaviourAction::DialPeer {
                    peer_id: peer.peer_id.clone(),
                    condition: Default::default(),
                });

                let mut address_hints = VecDeque::new();
                if let Some(address) = peer.address_hint {
                    address_hints.push_back(address);
                }

                let pending_events = vec![OutboundConfig::new(swap_to_announce)];

                entry.insert(ConnectionState::Connecting {
                    pending_events,
                    address_hints,
                });
            }
            Entry::Occupied(mut entry) => {
                let connection_state = entry.get_mut();

                match connection_state {
                    ConnectionState::Connecting {
                        pending_events,
                        address_hints,
                    } => {
                        pending_events.push(OutboundConfig::new(swap_to_announce));
                        if let Some(address) = peer.address_hint {
                            // We push to the front because we consider the new address to be the
                            // most likely one to succeed. The order of this queue is important
                            // when returning it from `addresses_of_peer()` because it will be tried
                            // by libp2p in the returned order.
                            address_hints.push_front(address);
                        }
                    }
                    ConnectionState::Connected { .. } => {
                        self.events
                            .push_back(NetworkBehaviourAction::NotifyHandler {
                                peer_id: peer.peer_id.clone(),
                                handler: NotifyHandler::Any,
                                event: OutboundConfig::new(swap_to_announce),
                            });
                    }
                }
            }
        }
    }

    /// Bob to await an announcement from Alice.
    ///
    /// This starts the announce protocol from Bob's perspective. In other
    /// words, he is going to wait for an announce message.
    pub fn await_announcement(&mut self, swap: SwapDigest, _from: PeerId) {
        self.awaiting_announcements.insert(swap);
    }

    /// Peer id and address information for connected peer nodes.
    pub fn connected_peers(&mut self) -> impl Iterator<Item = (PeerId, Vec<Multiaddr>)> {
        let addresses = self
            .connections
            .iter()
            .filter_map(|(peer, connection_state)| match connection_state {
                ConnectionState::Connecting { .. } => None,
                ConnectionState::Connected { addresses } => {
                    Some((peer.clone(), addresses.clone().into_iter().collect()))
                }
            })
            .collect::<Vec<_>>();

        addresses.into_iter()
    }
}

impl NetworkBehaviour for Announce {
    type ProtocolsHandler = Handler;
    type OutEvent = BehaviourOutEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Handler::default()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.connections
            .iter()
            .find_map(|(candidate, addresses)| {
                if candidate == peer_id {
                    Some(addresses)
                } else {
                    None
                }
            })
            .map(|connection_state| match connection_state {
                ConnectionState::Connecting { address_hints, .. } => {
                    let addresses: Vec<Multiaddr> = address_hints.clone().into();
                    addresses
                }
                ConnectionState::Connected { addresses } => addresses.iter().cloned().collect(),
            })
            .unwrap_or_else(Vec::new)
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        endpoint: &ConnectedPoint,
    ) {
        tracing::debug!("connected to {} at {:?}", peer_id, endpoint);

        let address = match endpoint {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        match self.connections.entry(peer_id.clone()) {
            Entry::Occupied(entry) => {
                let connection_state = entry.remove();

                match connection_state {
                    ConnectionState::Connected { mut addresses } => {
                        addresses.insert(address.clone());
                        self.connections
                            .insert(peer_id.clone(), ConnectionState::Connected { addresses });
                    }
                    ConnectionState::Connecting {
                        pending_events,
                        address_hints: _we_no_longer_care_at_this_stage,
                    } => {
                        for event in pending_events {
                            self.events
                                .push_back(NetworkBehaviourAction::NotifyHandler {
                                    peer_id: peer_id.clone(),
                                    handler: NotifyHandler::Any,
                                    event,
                                })
                        }

                        let mut addresses = HashSet::new();
                        addresses.insert(address.clone());

                        self.connections
                            .insert(peer_id.clone(), ConnectionState::Connected { addresses });
                    }
                }
            }
            Entry::Vacant(entry) => {
                let mut addresses = HashSet::new();
                addresses.insert(address.clone());

                entry.insert(ConnectionState::Connected { addresses });
            }
        }
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        endpoint: &ConnectedPoint,
    ) {
        tracing::debug!("disconnected from {} at {:?}", peer_id, endpoint);

        let address = match endpoint {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        if let Some(ConnectionState::Connected { mut addresses }) = self.connections.remove(peer_id)
        {
            addresses.remove(&address);

            if !addresses.is_empty() {
                self.connections
                    .insert(peer_id.clone(), ConnectionState::Connected { addresses });
            }
        }
    }

    fn inject_event(&mut self, peer: PeerId, _: ConnectionId, event: HandlerEvent) {
        match event {
            HandlerEvent::ReceivedConfirmation(Confirmed {
                swap_id,
                swap_digest,
            }) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::Confirmed {
                        peer,
                        swap_id,
                        swap_digest,
                    },
                ));
            }
            HandlerEvent::AwaitingConfirmation(sender) => {
                let swap_digest = sender.swap_digest.clone();

                if self.awaiting_announcements.contains(&swap_digest) {
                    let swap_id = SharedSwapId::default();

                    tokio::spawn(sender.send(swap_id));

                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                        BehaviourOutEvent::Confirmed {
                            peer,
                            swap_id,
                            swap_digest,
                        },
                    ))
                }
            }
            HandlerEvent::Error(error) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::Error { peer, error },
                ));
            }
        }
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // We trust in libp2p to poll us.
        Poll::Pending
    }
}

#[derive(Debug)]
enum ConnectionState {
    Connected {
        addresses: HashSet<Multiaddr>,
    },
    Connecting {
        // Vec is fine here, we iterate over this to remove items.
        pending_events: Vec<OutboundConfig>,
        // VecDeque because we push new addresses to the front.
        address_hints: VecDeque<Multiaddr>,
    },
}

/// Event emitted  by the `Announce` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    Confirmed {
        /// The peer (Bob) that the swap has been announced to.
        peer: PeerId,
        /// The swap id returned by the peer (Bob).
        swap_id: SharedSwapId,
        /// The swap_digest
        swap_digest: SwapDigest,
    },

    /// Error while attempting to announce swap to the remote.
    Error {
        /// The peer with whom the error originated.
        peer: PeerId,
        /// The error that occurred.
        error: handler::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::test_swarm;
    use futures::future;
    use libp2p::swarm::SwarmEvent;
    use std::{future::Future, time::Duration};

    #[tokio::test]
    async fn given_bob_awaits_an_announcements_when_alice_sends_one_then_swap_is_confirmed() {
        let (mut alice_swarm, _, alice_id) = test_swarm::new(Announce::default());
        let (mut bob_swarm, bob_addr, bob_id) = test_swarm::new(Announce::default());
        let swap_digest = SwapDigest::random();

        bob_swarm.await_announcement(swap_digest.clone(), alice_id.clone());
        alice_swarm.announce_swap(swap_digest, DialInformation {
            peer_id: bob_id.clone(),
            address_hint: Some(bob_addr),
        });

        assert_both_confirmed(alice_swarm.next(), bob_swarm.next()).await;
    }

    #[tokio::test]
    async fn given_alice_announces_swap_when_bob_awaits_it_within_timeout_then_swap_is_confirmed() {
        let incoming_announcement_buffer_expiry = Duration::from_secs(5);

        let (mut alice_swarm, _, alice_id) = test_swarm::new(Announce::default());
        let (mut bob_swarm, bob_addr, bob_id) =
            test_swarm::new(Announce::new(incoming_announcement_buffer_expiry));
        let swap_digest = SwapDigest::random();

        alice_swarm.announce_swap(swap_digest.clone(), DialInformation {
            peer_id: bob_id.clone(),
            address_hint: Some(bob_addr),
        });
        let bob_event = async {
            // wait until Alice established a connection
            loop {
                if let SwarmEvent::ConnectionEstablished { .. } = bob_swarm.next_event().await {
                    break;
                }
            }

            // poll Bob's swarm for another second. we don't care about the result, we just
            // want all events to be processed
            loop {
                if let Err(_) = tokio::time::timeout(Duration::from_secs(1), bob_swarm.next()).await
                {
                    break;
                }
            }

            bob_swarm.await_announcement(swap_digest, alice_id.clone());
            bob_swarm.next().await
        };

        assert_both_confirmed(alice_swarm.next(), bob_event).await;
    }

    async fn assert_both_confirmed(
        alice_event: impl Future<Output = BehaviourOutEvent>,
        bob_event: impl Future<Output = BehaviourOutEvent>,
    ) {
        let event_future = future::join(alice_event, bob_event);
        let (alice_event, bob_event) = tokio::time::timeout(Duration::from_secs(5), event_future)
            .await
            .expect("network behaviours should confirm the swap");

        match (alice_event, bob_event) {
            (
                BehaviourOutEvent::Confirmed {
                    swap_id: alice_event_swap_id,
                    swap_digest: alice_event_swap_digest,
                    ..
                },
                BehaviourOutEvent::Confirmed {
                    swap_id: bob_event_swap_id,
                    swap_digest: bob_event_swap_digest,
                    ..
                },
            ) => {
                assert_eq!(alice_event_swap_id, bob_event_swap_id);
                assert_eq!(alice_event_swap_digest, bob_event_swap_digest);
            }
            _ => panic!("expected both parties to confirm the swap"),
        }
    }
}
