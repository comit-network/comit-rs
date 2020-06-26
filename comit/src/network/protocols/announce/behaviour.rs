use crate::{
    network::{
        announce::{
            handler,
            handler::{Handler, HandlerEvent},
            protocol::OutboundConfig,
        },
        protocols::announce::ReplySubstream,
        SwapDigest,
    },
    DialInformation, SharedSwapId,
};
use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NegotiatedSubstream, NetworkBehaviour, NetworkBehaviourAction, NotifyHandler,
        PollParameters, ProtocolsHandler,
    },
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    task::{Context, Poll},
    time::{Duration, Instant},
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
#[derive(Debug, Default)]
pub struct Announce {
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<OutboundConfig, BehaviourOutEvent>>,
    /// Stores connection state for nodes we connect to.
    connections: HashMap<PeerId, ConnectionState>,
    /* sent_announcements: HashMap<SwapDigest, Instant>,
     *
     * awaiting_announcements: HashMap<SwapDigest, Instant>, */
}

impl Announce {
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
    pub fn await_announcement(&mut self, swap: SwapDigest, from: PeerId) {}

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

    fn inject_event(&mut self, peer_id: PeerId, _: ConnectionId, event: HandlerEvent) {
        match event {
            HandlerEvent::ReceivedConfirmation(confirmed) => {
                // self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                //     BehaviourOutEvent::ReceivedConfirmation {
                //         peer: peer_id,
                //         swap_id: confirmed.swap_id,
                //         swap_digest: confirmed.swap_digest,
                //     },
                // ));
            }
            HandlerEvent::AwaitingConfirmation(sender) => {
                // self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                //     BehaviourOutEvent::ReceivedAnnouncement {
                //         peer: peer_id,
                //         io: sender,
                //     },
                // ));
            }
            HandlerEvent::Error(error) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::Error {
                        peer: peer_id,
                        error,
                    },
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

    #[tokio::test]
    async fn given_bob_awaits_an_announcements_when_alice_sends_once_then_he_replies_with_shared_swap_id(
    ) {
        let (mut alice_swarm, _, alice_id) = test_swarm::new(Announce::default());
        let (mut bob_swarm, bob_addr, bob_id) = test_swarm::new(Announce::default());

        let swap_digest = SwapDigest::random();

        bob_swarm.await_announcement(swap_digest.clone(), alice_id.clone());
        alice_swarm.announce_swap(swap_digest, DialInformation {
            peer_id: bob_id.clone(),
            address_hint: Some(bob_addr),
        });

        let event_future = future::join(alice_swarm.next(), bob_swarm.next());
        let (alice_event, bob_event) = tokio::time::timeout(Duration::from_secs(2), event_future)
            .await
            .expect("network behaviours should confirm the swap");

        match (alice_event, bob_event) {
            (
                BehaviourOutEvent::Confirmed {
                    peer: alice_event_peer,
                    swap_id: alice_event_swap_id,
                    swap_digest: alice_event_swap_digest,
                },
                BehaviourOutEvent::Confirmed {
                    peer: bob_event_peer,
                    swap_id: bob_event_swap_id,
                    swap_digest: bob_event_swap_digest,
                },
            ) => {
                assert_eq!(alice_event_peer, bob_id);
                assert_eq!(bob_event_peer, alice_id);
                assert_eq!(alice_event_swap_id, bob_event_swap_id);
                assert_eq!(alice_event_swap_digest, bob_event_swap_digest);
            }
            _ => panic!("expected both parties to confirm the swap"),
        }
    }
}
