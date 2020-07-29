mod makerbook;
mod order;
mod order_source;
mod orders;
pub mod take_order;

use crate::SharedSwapId;
use libp2p::{
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage, ResponseChannel,
    },
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use std::{
    collections::VecDeque,
    task::{Context, Poll},
    time::Duration,
};

use makerbook::Makerbook;
pub use order::*;
use order_source::*;
use take_order::{Confirmation, TakeOrderCodec, TakeOrderProtocol};

pub use self::{order::*, orders::*};

/// The time we wait for a take order request to be confirmed or denied.
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// The Orderbook libp2p network behaviour.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "my_poll")]
#[allow(missing_debug_implementations)]
pub struct Orderbook {
    makerbook: Makerbook,
    order_source: OrderSource,
    take_order: RequestResponse<TakeOrderCodec>,

    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    orders: Orders,
}

impl Orderbook {
    /// Construct a new orderbook for this node using the node's peer ID.
    pub fn new(me: PeerId) -> Orderbook {
        let mut config = RequestResponseConfig::default();
        config.set_request_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS));
        let behaviour = RequestResponse::new(
            TakeOrderCodec::default(),
            vec![(TakeOrderProtocol, ProtocolSupport::Full)],
            config,
        );

        Orderbook {
            makerbook: Makerbook::new(me.clone()),
            order_source: OrderSource::default(),
            take_order: behaviour,
            events: VecDeque::new(),
            orders: Orders::new(me),
        }
    }

    /// Declare oneself to the network as a maker.
    pub fn declare_as_maker(&mut self) {
        self.makerbook.login();
    }

    /// Announce retraction of oneself as a maker, undoes `declare_as_maker()`.
    pub fn retract(&mut self) {
        self.makerbook.logout();
        self.orders.clear_own_orders();
    }

    /// Make a new order.
    ///
    /// The resulting order will eventually be visible to other peers.
    pub fn make(&mut self, new_order: NewOrder) -> OrderId {
        self.orders.new_order(new_order)
    }

    /// Cancel an order we previously made.
    pub fn cancel(&mut self, id: OrderId) {
        self.orders.cancel(id);
    }

    /// Take an order, called by Alice i.e., the taker.
    pub fn take(&mut self, order_id: OrderId) -> anyhow::Result<Order> {
        let order = self
            .orders
            .theirs()
            .find(|order| order.id == order_id)
            .ok_or_else(|| OrderNotFound(order_id))?;
        let maker = &order.maker;

        tracing::info!("attempting to take order {} from maker {}", order_id, maker);

        self.take_order.send_request(maker, order_id);

        Ok(order.clone())
    }

    /// Confirm a take order request, called by Bob i.e., the maker.
    pub fn confirm(
        &mut self,
        order_id: OrderId,
        channel: ResponseChannel<Confirmation>,
        peer_id: PeerId,
    ) {
        let shared_swap_id = SharedSwapId::default();
        tracing::debug!(
            "confirming take order request with swap id: {}",
            shared_swap_id
        );

        self.take_order.send_response(channel, Confirmation {
            shared_swap_id,
            order_id,
        });

        self.events
            .push_back(BehaviourOutEvent::TakeOrderConfirmation {
                peer_id,
                order_id,
                shared_swap_id,
            });
    }

    /// Deny a take order request, called by Bob i.e., the maker.
    pub fn deny(
        &mut self,
        peer_id: PeerId,
        order_id: OrderId,
        channel: ResponseChannel<Confirmation>,
    ) {
        self.events
            .push_back(BehaviourOutEvent::Failed { peer_id, order_id });
        std::mem::drop(channel); // triggers an error on the other end
    }

    pub fn orders(&self) -> &Orders {
        &self.orders
    }

    pub fn orders_mut(&mut self) -> &mut Orders {
        &mut self.orders
    }

    fn my_poll<BIE>(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<BIE, BehaviourOutEvent>> {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
#[error("order {0} not found in orderbook")]
pub struct OrderNotFound(OrderId);

/// Event emitted  by the `Orderbook` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    /// Event emitted within Bob's node when a take order request is received.
    TakeOrderRequest {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// Channel to send a confirm/deny response on.
        response_channel: ResponseChannel<Confirmation>,
        /// The ID of the order peer wants to take.
        order_id: OrderId,
    },
    /// Event emitted in both Alice and Bob's node when a take order is
    /// confirmed.
    TakeOrderConfirmation {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// The ID of the order taken.
        order_id: OrderId,
        /// Identifier for the swap, used by the COMIT communication protocols.
        shared_swap_id: SharedSwapId,
    },
    /// Event emitted in Bob's node when a take order fails, for Alice we just
    /// close the channel to signal the error.
    Failed {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// The ID of the order peer wanted to take.
        order_id: OrderId,
    },
}

impl NetworkBehaviourEventProcess<makerbook::BehaviourOutEvent> for Orderbook {
    fn inject_event(&mut self, event: makerbook::BehaviourOutEvent) {
        match event {
            makerbook::BehaviourOutEvent::Logout { peer } => {
                self.order_source.stop_getting_orders_from(&peer);
                self.orders.remove_all_from(&peer)
            }
        }
    }
}

impl NetworkBehaviourEventProcess<order_source::BehaviourOutEvent> for Orderbook {
    fn inject_event(&mut self, event: order_source::BehaviourOutEvent) {
        match event {
            order_source::BehaviourOutEvent::GetOrdersRequest { response_handle } => {
                self.order_source
                    .send_orders(response_handle, self.orders.ours().cloned().collect());
            }
            order_source::BehaviourOutEvent::RetrievedOrders { maker, orders } => {
                if !orders.is_empty() {
                    self.orders.replace(maker, orders);
                }
            }
            order_source::BehaviourOutEvent::MakerIsGone { maker } => {
                self.orders.remove_all_from(&maker);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<OrderId, Confirmation>> for Orderbook {
    fn inject_event(&mut self, event: RequestResponseEvent<OrderId, Confirmation>) {
        match event {
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Request {
                        request: order_id,
                        channel: response_channel,
                    },
            } => {
                if self.orders.is_ours(order_id) {
                    tracing::info!("received take order request for order {}", order_id);
                    self.events.push_back(BehaviourOutEvent::TakeOrderRequest {
                        peer_id,
                        response_channel,
                        order_id,
                    });
                } else {
                    tracing::info!(
                        "received take order request for order {} we never published",
                        order_id
                    );
                }
            }
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Response {
                        request_id: _,
                        response:
                            Confirmation {
                                shared_swap_id,
                                order_id,
                            },
                    },
            } => {
                self.events
                    .push_back(BehaviourOutEvent::TakeOrderConfirmation {
                        peer_id,
                        order_id,
                        shared_swap_id,
                    });
            }
            RequestResponseEvent::OutboundFailure { error, .. } => {
                tracing::warn!("outbound failure: {:?}", error);
            }
            RequestResponseEvent::InboundFailure { error, .. } => {
                tracing::warn!("inbound failure: {:?}", error);
            }
        }
    }
}
