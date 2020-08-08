mod makerbook;
mod order;
mod order_source;
mod orders;
pub mod take_order;

use crate::{asset, SharedSwapId};
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
use take_order::{
    Confirmation, TakeBtcDaiOrderCodec, TakeBtcDaiOrderProtocol, TakeBtcDaiOrderRequest,
};

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
    take_order: RequestResponse<TakeBtcDaiOrderCodec>,

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
            TakeBtcDaiOrderCodec::default(),
            vec![(TakeBtcDaiOrderProtocol, ProtocolSupport::Full)],
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
    pub fn take(
        &mut self,
        order_id: OrderId,
        partial_take_quantity: Option<asset::Bitcoin>,
    ) -> anyhow::Result<Order> {
        let order = self
            .orders
            .theirs()
            .find(|order| order.id == order_id)
            .ok_or_else(|| OrderNotFound(order_id))?;

        let order_amount = order.quantity;
        let take_amount = partial_take_quantity.unwrap_or(order_amount);

        let maker = &order.maker;

        if take_amount > order_amount {
            Err(anyhow::Error::from(PartialTakeAmountTooLarge(
                take_amount,
                order_amount,
                order_id,
            )))
        } else {
            tracing::info!("attempting to take order {} from maker {}", order_id, maker);
            self.take_order.send_request(maker, TakeBtcDaiOrderRequest {
                order_id,
                quantity: take_amount,
            });

            Ok(order.clone())
        }
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
pub struct OrderNotFound(pub OrderId);

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
#[error("partial take amount {0}, is greater than amount {1} specified in order {2}")]
pub struct PartialTakeAmountTooLarge(asset::Bitcoin, asset::Bitcoin, OrderId);

/// Event emitted  by the `Orderbook` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    /// Event emitted within Bob's node when a take order request is received.
    TakeOrderRequest {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// The amount of the order the taker wishes to fill
        quantity: asset::Bitcoin,
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

impl NetworkBehaviourEventProcess<RequestResponseEvent<TakeBtcDaiOrderRequest, Confirmation>>
    for Orderbook
{
    fn inject_event(&mut self, event: RequestResponseEvent<TakeBtcDaiOrderRequest, Confirmation>) {
        match event {
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Request {
                        request,
                        channel: response_channel,
                    },
            } => {
                if self.orders.is_ours(request.order_id) {
                    tracing::info!("received take order request for order {}", request.order_id);
                    self.events.push_back(BehaviourOutEvent::TakeOrderRequest {
                        peer_id,
                        quantity: request.quantity,
                        response_channel,
                        order_id: request.order_id,
                    });
                } else {
                    tracing::info!(
                        "received take order request for order {} we never published",
                        request.order_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asset, identity, ledger,
        network::{
            orderbook::BehaviourOutEvent,
            test::{await_events_or_timeout, new_connected_swarm_pair},
        },
    };
    use libp2p::Swarm;
    use std::str::FromStr;

    fn new_order() -> NewOrder {
        let token_contract =
            identity::Ethereum::from_str("0xc5549e335b2786520f4c5d706c76c9ee69d0a028").unwrap();

        NewOrder {
            position: Position::Buy,
            quantity: asset::Bitcoin::meaningless_test_value(),
            bitcoin_ledger: ledger::Bitcoin::Regtest,
            // TODO: Add test function helper to return expiry value.
            bitcoin_absolute_expiry: 100,
            price: BtcDaiRate(100),
            token_contract,
            ethereum_ledger: ledger::Ethereum::default(),
            ethereum_absolute_expiry: 100,
        }
    }

    #[tokio::test]
    async fn take_order_request_confirmation() {
        let (mut alice, mut bob) = new_connected_swarm_pair(Orderbook::new).await;

        // Poll to trigger the initial get_orders request/response (done on dial).
        poll_no_event(&mut bob.swarm).await;
        poll_no_event(&mut alice.swarm).await;

        let _order_id = bob.swarm.make(new_order());

        // Poll to trigger get_orders request/response messages
        poll_no_event(&mut bob.swarm).await;
        poll_no_event(&mut alice.swarm).await;

        let alice_order = alice
            .swarm
            .orders()
            .all()
            .next()
            .cloned()
            .expect("Alice has no orders");

        alice
            .swarm
            .take(alice_order.id, None)
            .expect("failed to take order");

        // Poll to trigger take_order request/response messages.
        poll_no_event(&mut alice.swarm).await;
        let bob_event = tokio::time::timeout(Duration::from_secs(2), bob.swarm.next())
            .await
            .expect("failed to get TakeOrderRequest event");

        let (alice_peer_id, channel, order_id) = match bob_event {
            BehaviourOutEvent::TakeOrderRequest {
                peer_id,
                response_channel,
                order_id,
                ..
            } => (peer_id, response_channel, order_id),
            _ => panic!("unexepected bob event"),
        };
        bob.swarm.confirm(order_id, channel, alice_peer_id);

        let (alice_event, bob_event) =
            await_events_or_timeout(alice.swarm.next(), bob.swarm.next()).await;
        match (alice_event, bob_event) {
            (
                BehaviourOutEvent::TakeOrderConfirmation {
                    peer_id: alice_got_peer_id,
                    order_id: alice_got_order_id,
                    shared_swap_id: alice_got_swap_id,
                },
                BehaviourOutEvent::TakeOrderConfirmation {
                    peer_id: bob_got_peer_id,
                    order_id: bob_got_order_id,
                    shared_swap_id: bob_got_swap_id,
                },
            ) => {
                assert_eq!(alice_got_peer_id, bob.peer_id);
                assert_eq!(bob_got_peer_id, alice.peer_id);

                assert_eq!(alice_got_order_id, alice_order.id);
                assert_eq!(bob_got_order_id, alice_got_order_id);

                assert_eq!(alice_got_swap_id, bob_got_swap_id);
            }
            _ => panic!("failed to get take order confirmation"),
        }
    }

    // Poll the swarm for some time, we don't expect any events though.
    async fn poll_no_event(swarm: &mut Swarm<Orderbook>) {
        let delay = Duration::from_secs(2);

        while let Ok(event) = tokio::time::timeout(delay, swarm.next()).await {
            panic!("unexpected event emitted: {:?}", event)
        }
    }
}
