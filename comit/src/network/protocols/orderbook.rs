mod order;
mod orders;
mod quote;
mod take_order;

use crate::{
    network::protocols::orderbook::take_order::{Response, TakeOrderCodec, TakeOrderProtocol},
    SharedSwapId,
};

use anyhow::bail;
use libp2p::{
    core::either::EitherOutput,
    gossipsub,
    gossipsub::{Gossipsub, GossipsubEvent, GossipsubRpc, Topic},
    request_response::{
        handler::RequestProtocol, ProtocolSupport, RequestResponse, RequestResponseConfig,
        RequestResponseEvent, RequestResponseMessage, ResponseChannel,
    },
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{hash_map::DefaultHasher, VecDeque},
    fmt,
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
    task::{Context, Poll},
    time::Duration,
};

pub use self::{order::*, orders::*, quote::*};

/// String representing the BTC/DAI trading pair.
const BTC_DAI: &str = "BTC/DAI";

/// The time we wait for a take order request to be confirmed or denied.
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// The Orderbook libp2p network behaviour.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct Orderbook {
    gossipsub: Gossipsub,
    take_order: RequestResponse<TakeOrderCodec>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    orders: Orders,
    #[behaviour(ignore)]
    pub peer_id: PeerId,
}

impl Orderbook {
    /// Construct a new orderbook for this node using the node's peer ID.
    pub fn new(peer_id: PeerId) -> Orderbook {
        let message_id_fn = |message: &gossipsub::GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId(s.finish().to_string())
        };

        let config = gossipsub::GossipsubConfigBuilder::new()
            .heartbeat_interval(Duration::from_secs(1))
            .message_id_fn(message_id_fn) // No two messages of the same content will be propagated.
            .build();
        let gossipsub = Gossipsub::new(peer_id.clone(), config);

        let mut config = RequestResponseConfig::default();
        config.set_request_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS));
        let behaviour = RequestResponse::new(
            TakeOrderCodec::default(),
            vec![(TakeOrderProtocol, ProtocolSupport::Full)],
            config,
        );

        let mut orderbook = Orderbook {
            peer_id,
            gossipsub,
            take_order: behaviour,
            orders: Orders::default(),
            events: VecDeque::new(),
        };

        // Since we only support a single trading pair topic just subscribe to it now.
        orderbook
            .gossipsub
            .subscribe(Topic::new(BTC_DAI.to_string()));

        orderbook
    }

    /// Create and publish a new 'make' order. Called by Bob i.e. the maker.
    pub fn make(&mut self, maker: PeerId, order: BtcDaiOrder) -> anyhow::Result<OrderId> {
        let ser = bincode::serialize(&Message::CreateOrder(order))?;
        let topic = order.to_topic();
        self.gossipsub.publish(&topic, ser);

        self.orders.insert(maker, order)?;

        Ok(order.id)
    }

    /// Take an order, called by Alice i.e., the taker.
    pub fn take(&mut self, id: OrderId) -> anyhow::Result<()> {
        let maker = self.orders.maker(&id).ok_or_else(|| OrderNotFound(id))?;

        if !self.orders.is_live(&id) {
            bail!("order is dead: {}", id);
        }

        self.take_order.send_request(&maker, id);

        Ok(())
    }

    /// Get the ID of the node that published this order.
    pub fn maker(&self, order_id: OrderId) -> Option<PeerId> {
        self.orders.maker(&order_id)
    }

    /// Confirm a take order request, called by Bob i.e., the maker.
    /// Does _not_ remove the order from the order book.
    pub fn confirm(
        &mut self,
        order_id: OrderId,
        taker: PeerId,
        channel: ResponseChannel<Response>,
    ) {
        let shared_swap_id = SharedSwapId::default();
        tracing::debug!(
            "confirming take order request with swap id: {}",
            shared_swap_id
        );

        self.take_order
            .send_response(channel, Response::Confirmation {
                order_id,
                shared_swap_id,
            });

        self.events
            .push_back(BehaviourOutEvent::TakeOrderConfirmation {
                peer_id: taker,
                order_id,
                shared_swap_id,
            });
    }

    /// Deny a take order request, called by Bob i.e., the maker.
    pub fn deny(&mut self, order_id: OrderId, taker: PeerId, channel: ResponseChannel<Response>) {
        self.events.push_back(BehaviourOutEvent::Failed {
            peer_id: taker,
            order_id,
        });
        self.take_order.send_response(channel, Response::Error);
    }

    /// Get a list of all the orders known to this node.
    pub fn get_orders(&self) -> Vec<BtcDaiOrder> {
        self.orders.get_orders()
    }

    /// Get the order matching `id` if known to this node.
    pub fn get_order(&self, id: &OrderId) -> Option<BtcDaiOrder> {
        self.orders.get_order(id)
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            EitherOutput<GossipsubRpc, RequestProtocol<TakeOrderCodec>>,
            BehaviourOutEvent,
        >,
    > {
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }
}

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
#[error("order {0} not found in orderbook")]
pub struct OrderNotFound(OrderId);

/// MakerId is a PeerId wrapper so we control serialization/deserialization.
#[derive(Debug, Clone, PartialEq)]
struct MakerId(PeerId);

impl From<PeerId> for MakerId {
    fn from(id: PeerId) -> Self {
        MakerId(id)
    }
}

impl From<MakerId> for PeerId {
    fn from(id: MakerId) -> Self {
        id.0
    }
}

impl Display for MakerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl Serialize for MakerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = self.0.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for MakerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let peer_id = PeerId::from_str(&string).map_err(D::Error::custom)?;

        Ok(MakerId(peer_id))
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
    CreateOrder(BtcDaiOrder),
    DeleteOrder(OrderId),
}

impl fmt::Debug for Orderbook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Orderbook")
            .field("peer_id", &self.peer_id)
            .field("orders", &self.orders)
            .finish()
    }
}

/// Event emitted  by the `Orderbook` behaviour.
#[derive(Debug)]
pub enum BehaviourOutEvent {
    /// Event emitted within Bob's node when a take order request is received.
    TakeOrderRequest {
        /// The peer from whom request originated.
        peer_id: PeerId,
        /// Channel to send a confirm/deny response on.
        response_channel: ResponseChannel<Response>,
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

impl NetworkBehaviourEventProcess<GossipsubEvent> for Orderbook {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message(peer_id, _message_id, message) = event {
            let decoded: Message = match bincode::deserialize(&message.data[..]) {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::warn!("deserialization of gossipsub message failed: {}", e);
                    return;
                }
            };

            match decoded {
                Message::CreateOrder(order) => {
                    let id = order.id;
                    if self.orders.insert(peer_id, order).is_err() {
                        tracing::warn!("insert failed, order exists: {}", id);
                    }
                }
                // TODO: Add cancel/taken instead of just kill.
                Message::DeleteOrder(order_id) => {
                    let _ = self.orders.kill_order(peer_id, &order_id);
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<OrderId, Response>> for Orderbook {
    fn inject_event(&mut self, event: RequestResponseEvent<OrderId, Response>) {
        match event {
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Request {
                        request: order_id,
                        channel: response_channel,
                    },
            } => match self.get_order(&order_id) {
                Some(_) => {
                    tracing::info!("received take order request");
                    self.events.push_back(BehaviourOutEvent::TakeOrderRequest {
                        peer_id,
                        response_channel,
                        order_id,
                    });
                }
                None => tracing::info!("received take order request for non-existent order"),
            },
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Response {
                        request_id: _,
                        response:
                            Response::Confirmation {
                                order_id,
                                shared_swap_id,
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
            RequestResponseEvent::Message {
                peer: peer_id,
                message:
                    RequestResponseMessage::Response {
                        request_id: _,
                        response: Response::Error,
                    },
            } => {
                // This should be unreachable because we close the channel on error.
                tracing::error!("received take order response error from peer: {}", peer_id);
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
    use crate::network::test::{await_events_or_timeout, new_connected_swarm_pair};
    use libp2p::Swarm;
    use spectral::prelude::*;

    #[tokio::test]
    async fn take_order_request_confirmation() {
        // arrange

        let (mut alice, mut bob) = new_connected_swarm_pair(Orderbook::new).await;
        let bob_order = BtcDaiOrder::meaningless_test_value();

        // act

        // Trigger subscription to BCT/DAI topic.
        poll_no_event(&mut alice.swarm).await;
        poll_no_event(&mut bob.swarm).await;

        let _ = bob
            .swarm
            .make(bob.peer_id.clone(), bob_order)
            .expect("order id should exist");

        // Trigger publish and receipt of order.
        poll_no_event(&mut bob.swarm).await;
        poll_no_event(&mut alice.swarm).await;

        let alice_order = alice
            .swarm
            .get_orders()
            .first()
            .cloned()
            .expect("Alice has no orders");

        alice
            .swarm
            .take(alice_order.id)
            .expect("failed to take order");

        // Trigger request/response messages.
        poll_no_event(&mut alice.swarm).await;
        let bob_event = tokio::time::timeout(Duration::from_secs(2), bob.swarm.next())
            .await
            .expect("failed to get TakeOrderRequest event");

        let (alice_peer_id, channel, order_id) = match bob_event {
            BehaviourOutEvent::TakeOrderRequest {
                peer_id,
                response_channel,
                order_id,
            } => (peer_id, response_channel, order_id),
            _ => panic!("unexepected bob event"),
        };
        bob.swarm.confirm(order_id, alice_peer_id, channel);

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

    #[test]
    fn maker_id_serializes_as_expected() {
        let given = "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY".to_string();
        let peer_id = PeerId::from_str(&given).expect("failed to parse peer id");
        let maker_id = MakerId(peer_id);

        let want = format!("\"{}\"", given);
        let got = serde_json::to_string(&maker_id).expect("failed to serialize peer id");

        assert_that(&got).is_equal_to(want);
    }

    #[test]
    fn maker_id_serialization_roundtrip_test() {
        let s = "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY".to_string();
        let peer_id = PeerId::from_str(&s).expect("failed to parse peer id");
        let maker_id = MakerId::from(peer_id);

        let json = serde_json::to_string(&maker_id).expect("failed to serialize peer id");
        let rinsed: MakerId = serde_json::from_str(&json).expect("failed to deserialize peer id");

        assert_that(&maker_id).is_equal_to(rinsed);
    }
}
