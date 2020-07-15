mod take_order;

use crate::{
    asset,
    network::protocols::orderbook::take_order::{Response, TakeOrderCodec, TakeOrderProtocol},
    SharedSwapId,
};
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
    collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque},
    fmt,
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
    task::{Context, Poll},
    time::Duration,
};
use uuid::Uuid;

// We only support a single topic at the moment.
const TOPIC: &str = "Herc20Hbit";
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
    orders: HashMap<OrderId, Order>,
    #[behaviour(ignore)]
    trading_pairs: HashSet<TradingPairTopic>,
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
            trading_pairs: HashSet::new(),
            orders: HashMap::new(),
            events: VecDeque::new(),
        };

        orderbook.gossipsub.subscribe(Makers::topic());

        // Since we only support a single trading pair topic just subscribe to it now.
        orderbook.gossipsub.subscribe(Topic::new(TOPIC.to_string()));

        orderbook
    }

    /// Create and publish a new 'make' order. Called by Bob i.e. the maker.
    pub fn make(&mut self, order: Order) -> anyhow::Result<OrderId> {
        let order_id = order.id;
        let ser = bincode::serialize(&Message::CreateOrder(order.clone()))?;
        let topic = order.topic(&self.peer_id);

        self.gossipsub.publish(&topic, ser);
        tracing::info!("published order: {}", order_id);

        self.orders.insert(order_id, order);

        Ok(order_id)
    }

    /// Take an order, called by Alice i.e., the taker.
    /// Does _not_ remove the order from the order book.
    pub fn take(&mut self, order_id: OrderId) -> anyhow::Result<()> {
        let maker_id = self
            .maker_id(order_id)
            .ok_or_else(|| OrderbookError::OrderNotFound(order_id))?;

        self.take_order.send_request(&maker_id.into(), order_id);

        Ok(())
    }

    /// Get the ID of the node that published this order.
    fn maker_id(&self, order_id: OrderId) -> Option<MakerId> {
        self.orders.get(&order_id).map(|order| order.maker.clone())
    }

    /// Confirm a take order request, called by Bob i.e., the maker.
    /// Does _not_ remove the order from the order book.
    pub fn confirm(&mut self, order_id: OrderId, channel: ResponseChannel<Response>) {
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
                order_id,
                shared_swap_id,
            });
    }

    /// Deny a take order request, called by Bob i.e., the maker.
    pub fn deny(&mut self, peer_id: PeerId, order_id: OrderId, channel: ResponseChannel<Response>) {
        self.events
            .push_back(BehaviourOutEvent::Failed { peer_id, order_id });
        self.take_order.send_response(channel, Response::Error);
    }

    /// Get a list of all orders known to this node.
    pub fn get_orders(&self) -> Vec<Order> {
        self.orders.values().cloned().collect()
    }

    /// Get the order matching `id` if known to this node.
    pub fn get_order(&self, id: &OrderId) -> Option<Order> {
        self.orders.get(id).cloned()
    }

    pub fn get_trading_pairs(&self) -> Vec<TradingPairTopic> {
        self.trading_pairs.iter().cloned().collect()
    }

    /// Add a trading pair, called when we receive a topic via gossipsub.
    pub fn add_trading_pair(&mut self, topic: &TradingPairTopic) {
        self.trading_pairs.insert(topic.clone());
    }

    /// Subscribe to a trading pair topic from peer.
    pub fn subscribe(&mut self, peer: PeerId, trading_pair: TradingPair) -> anyhow::Result<()> {
        let topic = TradingPairTopic::new(peer, trading_pair);
        self.trading_pairs.insert(topic.clone());
        self.gossipsub.subscribe(topic.to_topic());
        Ok(())
    }

    /// Unsubscribe from a trading pair topic from peer.
    pub fn unsubscribe(&mut self, peer: PeerId, trading_pair: TradingPair) -> anyhow::Result<()> {
        let topic = TradingPairTopic::new(peer, trading_pair);
        self.trading_pairs.remove(&topic);
        self.gossipsub.unsubscribe(topic.to_topic());
        Ok(())
    }

    // Ideally this step should be executed automatically before making an order.
    // Unfortunately a brief delay is required to allow peers to acknowledge and
    // subscribe to the announced trading pair before publishing the order
    /// Announce a trading pair topic to the network.
    pub fn announce_trading_pair(&mut self, tp: TradingPair) -> anyhow::Result<()> {
        let topic = TradingPairTopic::new(self.peer_id.clone(), tp).to_topic();
        let ser = bincode::serialize(&Message::TradingPair(tp))?;
        self.gossipsub.subscribe(topic.clone());
        self.gossipsub.publish(&topic, ser);

        Ok(())
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

#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum OrderbookError {
    #[error("could not make order")]
    Make,
    #[error("could not take order because identities not found")]
    IdentitiesForOrderNotFound(OrderId),
    #[error("could not take order because not found")]
    OrderNotFound(OrderId),
    #[error("could not subscribe to all peers for topic")]
    Subscribe,
    #[error("could not unsubscribe to all peers for topic")]
    UnSubscribe,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TradingPair {
    pub buy: SwapType,
    pub sell: SwapType,
}

impl TradingPair {
    pub fn to_topic(&self, peer: &PeerId) -> Topic {
        let trading_pair_topic = TradingPairTopic {
            peer: PeerId::into_bytes(peer.clone()),
            buy: self.buy,
            sell: self.sell,
        };
        trading_pair_topic.to_topic()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TradingPairTopic {
    peer: Vec<u8>,
    buy: SwapType,
    sell: SwapType,
}

impl TradingPairTopic {
    fn new(peer: PeerId, trading_pair: TradingPair) -> TradingPairTopic {
        TradingPairTopic {
            peer: PeerId::into_bytes(peer),
            buy: trading_pair.buy,
            sell: trading_pair.sell,
        }
    }
    fn to_topic(&self) -> Topic {
        Topic::new(TOPIC.to_string())
    }
}

pub type OrderId = Uuid;

/// MakerId is a PeerId wrapper so we control serialization/deserialization.
#[derive(Debug, Clone, PartialEq)]
pub struct MakerId(PeerId);

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub id: OrderId,
    pub maker: MakerId,
    pub buy: u64,
    pub sell: asset::Erc20,
    pub absolute_expiry: u32,
}

#[derive(Debug)]
pub struct NewOrder {
    pub buy: asset::Bitcoin,
    pub sell: asset::Erc20,
    pub absolute_expiry: u32,
}

impl Order {
    pub fn new(peer_id: PeerId, new_order: NewOrder) -> Self {
        Order {
            id: Uuid::new_v4(),
            maker: MakerId(peer_id),
            buy: new_order.buy.as_sat(),
            sell: new_order.sell,
            absolute_expiry: new_order.absolute_expiry,
        }
    }

    pub fn topic(&self, peer: &PeerId) -> Topic {
        TradingPair {
            buy: SwapType::Hbit,
            sell: SwapType::Herc20,
        }
        .to_topic(peer)
    }
}

/// Used to publish/subscribe available makers.
#[derive(Debug, Clone, Copy)]
pub struct Makers;

impl Makers {
    pub fn topic() -> Topic {
        Topic::new("makers".to_string())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum SwapType {
    Herc20,
    Hbit,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
    TradingPair(TradingPair),
    CreateOrder(Order),
    DeleteOrder(OrderId),
}

impl fmt::Debug for Orderbook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicOrderbook")
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
                    self.orders.insert(order.id, order);
                }
                Message::DeleteOrder(order_id) => {
                    self.orders.remove(&order_id);
                }
                Message::TradingPair(trading_pair) => {
                    self.add_trading_pair(&TradingPairTopic::new(peer_id.clone(), trading_pair));
                    self.gossipsub
                        .subscribe(TradingPairTopic::new(peer_id, trading_pair).to_topic());
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
                peer: _,
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
    use crate::{
        asset::{self, Erc20, Erc20Quantity},
        network::test::{await_events_or_timeout, new_connected_swarm_pair},
    };
    use libp2p::Swarm;
    use spectral::prelude::*;

    fn create_order(id: PeerId) -> Order {
        Order::new(id, NewOrder {
            buy: asset::Bitcoin::from_sat(100),
            sell: Erc20 {
                token_contract: Default::default(),
                quantity: Erc20Quantity::max_value(),
            },
            absolute_expiry: 100,
        })
    }

    #[tokio::test]
    async fn take_order_request_confirmation() {
        // arrange

        let (mut alice, mut bob) = new_connected_swarm_pair(Orderbook::new).await;
        let bob_order = create_order(bob.peer_id.clone());

        // act

        // Trigger subscription.
        poll_no_event(&mut alice.swarm).await;
        poll_no_event(&mut bob.swarm).await;

        let _ = bob
            .swarm
            .make(bob_order.clone())
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

        let (_peer_id, channel, order_id) = match bob_event {
            BehaviourOutEvent::TakeOrderRequest {
                peer_id,
                response_channel,
                order_id,
            } => (peer_id, response_channel, order_id),
            _ => panic!("unexepected bob event"),
        };
        bob.swarm.confirm(order_id, channel);

        let (alice_event, bob_event) =
            await_events_or_timeout(alice.swarm.next(), bob.swarm.next()).await;
        match (alice_event, bob_event) {
            (
                BehaviourOutEvent::TakeOrderConfirmation {
                    order_id: alice_got_order_id,
                    shared_swap_id: alice_got_swap_id,
                },
                BehaviourOutEvent::TakeOrderConfirmation {
                    order_id: bob_got_order_id,
                    shared_swap_id: bob_got_swap_id,
                },
            ) => {
                assert_eq!(alice_got_order_id, bob_order.id);
                assert_eq!(bob_got_order_id, alice_order.id);
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
    fn peer_id_serializes_as_expected() {
        let given = "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY".to_string();
        let peer_id = PeerId::from_str(&given).expect("failed to parse peer id");
        let maker_id = MakerId(peer_id);

        let want = format!("\"{}\"", given);
        let got = serde_json::to_string(&maker_id).expect("failed to serialize peer id");

        assert_that(&got).is_equal_to(want);
    }

    #[test]
    fn peer_id_serialization_roundtrip_test() {
        let s = "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY".to_string();
        let peer_id = PeerId::from_str(&s).expect("failed to parse peer id");
        let maker_id = MakerId::from(peer_id);

        let json = serde_json::to_string(&maker_id).expect("failed to serialize peer id");
        let rinsed: MakerId = serde_json::from_str(&json).expect("failed to deserialize peer id");

        assert_that(&maker_id).is_equal_to(rinsed);
    }
}
