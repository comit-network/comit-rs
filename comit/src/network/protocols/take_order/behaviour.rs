use crate::{
    network::protocols::{
        orderbook::OrderId,
        take_order::{
            handler::{self, Handler, HandlerEvent},
            protocol::OutboundConfig,
        },
        ReplySubstream, SwapDigest,
    },
    SharedSwapId,
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
};

/// Network behaviour that announces a swap to peer by sending a `swap_digest`
/// and receives the `swap_id` back.
#[derive(Debug)]
pub struct TakeOrder {
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<OutboundConfig, BehaviourOutEvent>>,
    /// Stores connection state for nodes we connect to.
    connections: HashMap<PeerId, ConnectionState>,
}

impl Default for TakeOrder {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
            connections: HashMap::new(),
        }
    }
}

impl TakeOrder {
    pub fn take(
        &mut self,
        order_id: OrderId,
        swap_digest: SwapDigest,
        maker: PeerId,
        maker_address: Multiaddr,
    ) {
        tracing::info!(
            "sending take order request with id: {:?} to peer: {:?} with addr: {:?}",
            order_id,
            maker,
            maker_address
        );
        match self.connections.entry(maker.clone()) {
            Entry::Vacant(entry) => {
                self.events.push_back(NetworkBehaviourAction::DialPeer {
                    peer_id: maker,
                    condition: Default::default(),
                });

                entry.insert(ConnectionState::Connecting {
                    pending_events: vec![OutboundConfig::new(order_id, swap_digest)],
                    address_hints: vec![maker_address].into(),
                });
            }
            Entry::Occupied(mut entry) => {
                let connection_state = entry.get_mut();

                match connection_state {
                    ConnectionState::Connecting {
                        pending_events,
                        address_hints,
                    } => {
                        pending_events.push(OutboundConfig::new(order_id, swap_digest));
                        // We push to the front because we consider the new address to be the
                        // most likely one to succeed. The order of this queue is important
                        // when returning it from `addresses_of_peer()` because it will be tried
                        // by libp2p in the returned order.
                        address_hints.push_front(maker_address);
                    }
                    ConnectionState::Connected { .. } => {
                        self.events
                            .push_back(NetworkBehaviourAction::NotifyHandler {
                                peer_id: maker,
                                handler: NotifyHandler::Any,
                                event: OutboundConfig::new(order_id, swap_digest),
                            });
                    }
                }
            }
        }
    }
}

impl NetworkBehaviour for TakeOrder {
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
            HandlerEvent::ReceivedTakeOrderRequest {
                order_id,
                reply_substream,
            } => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::TakeOrderRequest {
                        peer: peer_id,
                        order_id,
                        io: reply_substream,
                    },
                ));
            }
            HandlerEvent::ReceivedTakeOrderResponse(order_confirmed) => {
                self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                    BehaviourOutEvent::TakeOrderResponse {
                        peer: peer_id,
                        swap_digest: order_confirmed.swap_digest,
                        shared_swap_id: order_confirmed.swap_id,
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
    TakeOrderRequest {
        /// The peer that submitted the take order request
        peer: PeerId,
        order_id: OrderId,
        /// The substream to confirm the order
        io: ReplySubstream<NegotiatedSubstream>,
    },
    TakeOrderResponse {
        /// The peer that submitted the take order response
        peer: PeerId,
        /// The swap_digest of the order which the peer is wishes to take
        swap_digest: SwapDigest,
        /// The shared_swap_id of the order which the peer is wishes to take
        shared_swap_id: SharedSwapId,
    },
    Error {
        /// The peer with whom the error originated.
        peer: PeerId,
        /// The error that occurred.
        error: handler::Error,
    },
}
