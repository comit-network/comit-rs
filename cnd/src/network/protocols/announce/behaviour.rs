use crate::{
    network::{
        protocols::announce::{
            handler::{self, Handler, HandlerEvent},
            protocol::{OutboundConfig, ReplySubstream},
            SwapDigest,
        },
        DialInformation,
    },
    swap_protocols::SwapId,
};
use libp2p::{
    core::{ConnectedPoint, Multiaddr, PeerId},
    swarm::{
        NegotiatedSubstream, NetworkBehaviour, NetworkBehaviourAction, PollParameters,
        ProtocolsHandler,
    },
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    task::{Context, Poll},
};

/// Network behaviour that announces a swap to peer by sending a `swap_digest`
/// and receives the `swap_id` back.
#[derive(Debug)]
pub struct Announce {
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<OutboundConfig, BehaviourOutEvent>>,
    /// Stores connection state for nodes we connect to.
    connections: HashMap<PeerId, ConnectionState>,
}

impl Default for Announce {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
            connections: HashMap::new(),
        }
    }
}

impl Announce {
    /// Start the announce protocol.
    ///
    /// This is the entry point for Alice when wishing to start the announce
    /// protocol to announce a swap to Bob.  In libp2p parlance Alice is the
    /// dialer and Bob is the listener, `dial_info` is what is used to dial Bob.
    ///
    /// # Arguments
    ///
    /// * `swap_digest` - The swap to announce.
    /// * `dial_info` - The `PeerId` and address hint to dial to Bob's node.
    pub fn start_announce_protocol(&mut self, swap_digest: SwapDigest, dial_info: DialInformation) {
        tracing::info!("Announcing swap {} to {}", swap_digest, dial_info.peer_id);

        match self.connections.entry(dial_info.peer_id.clone()) {
            Entry::Vacant(entry) => {
                self.events.push_back(NetworkBehaviourAction::DialPeer {
                    peer_id: dial_info.peer_id.clone(),
                });

                let mut address_hints = VecDeque::new();
                if let Some(address) = dial_info.address_hint {
                    address_hints.push_back(address);
                }

                let pending_events = vec![OutboundConfig::new(swap_digest)];

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
                        pending_events.push(OutboundConfig::new(swap_digest));
                        if let Some(address) = dial_info.address_hint {
                            // We push to the front because we consider the new address to be the
                            // most likely one to succeed. The order of this queue is important
                            // when returning it from `addresses_of_peer()` because it will be tried
                            // by libp2p in the returned order.
                            address_hints.push_front(address);
                        }
                    }
                    ConnectionState::Connected { .. } => {
                        self.events.push_back(NetworkBehaviourAction::SendEvent {
                            peer_id: dial_info.peer_id.clone(),
                            event: OutboundConfig::new(swap_digest),
                        });
                    }
                }
            }
        }
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

    fn inject_connected(&mut self, peer_id: PeerId, endpoint: ConnectedPoint) {
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
                        addresses.insert(address);
                        self.connections
                            .insert(peer_id, ConnectionState::Connected { addresses });
                    }
                    ConnectionState::Connecting {
                        pending_events,
                        address_hints: _we_no_longer_care_at_this_stage,
                    } => {
                        for event in pending_events {
                            self.events.push_back(NetworkBehaviourAction::SendEvent {
                                peer_id: peer_id.clone(),
                                event,
                            })
                        }

                        let mut addresses = HashSet::new();
                        addresses.insert(address);

                        self.connections
                            .insert(peer_id, ConnectionState::Connected { addresses });
                    }
                }
            }
            Entry::Vacant(entry) => {
                let mut addresses = HashSet::new();
                addresses.insert(address);

                entry.insert(ConnectionState::Connected { addresses });
            }
        }
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, endpoint: ConnectedPoint) {
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

    fn inject_node_event(&mut self, peer_id: PeerId, event: HandlerEvent) {
        match event {
            HandlerEvent::ReceivedConfirmation(confirmed) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::ReceivedConfirmation {
                        peer: peer_id,
                        swap_id: confirmed.swap_id,
                        swap_digest: confirmed.swap_digest,
                    },
                ));
            }
            HandlerEvent::AwaitingConfirmation(sender) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::ReceivedAnnouncement {
                        peer: peer_id,
                        io: sender,
                    },
                ));
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
    /// This event created when a confirmation message containing a `swap_id` is
    /// received in response to an announce messagunlo  e containing a
    /// `swap_digest`. The Event contains both the swap id and
    /// the swap digest. The announce message is sent by Alice to Bob.
    ReceivedConfirmation {
        /// The peer (Bob) that the swap has been announced to.
        peer: PeerId,
        /// The swap_id returned by the peer (Bob).
        swap_id: SwapId,
        /// The swap_digest
        swap_digest: SwapDigest,
    },

    /// The event is created when a remote sends a `swap_digest`. The event
    /// contains a reply substream for the receiver to send back the
    /// `swap_id` that corresponds to the swap digest. Bob sends the
    /// confirmations message to Alice using the the reply substream.
    ReceivedAnnouncement {
        /// The peer (Alice) that the reply substream is connected to.
        peer: PeerId,
        /// The substream (inc. `swap_digest`) to reply on (i.e., send
        /// `swap_id`).
        io: ReplySubstream<NegotiatedSubstream>,
    },

    /// Error while attempting to announce swap to the remote.
    Error {
        /// The peer with whom the error originated.
        peer: PeerId,
        /// The error that occurred.
        error: handler::Error,
    },
}
