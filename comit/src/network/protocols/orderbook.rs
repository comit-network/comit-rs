mod take_order;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    time::Duration,
};
use uuid::Uuid;

use crate::{
    asset, identity,
    network::protocols::orderbook::take_order::{Response, TakeOrderProtocol},
};
use libp2p::{
    core::either::EitherOutput,
    gossipsub,
    gossipsub::{Gossipsub, GossipsubEvent, GossipsubRpc, Topic},
    multiaddr::Multiaddr,
    request_response::{
        handler::RequestProtocol, ProtocolSupport, RequestId, RequestResponse,
        RequestResponseConfig, RequestResponseEvent, RequestResponseMessage, ResponseChannel,
    },
    swarm::{NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    NetworkBehaviour, PeerId,
};
use serde::de::Error;
use std::{
    collections::VecDeque,
    str::FromStr,
    task::{Context, Poll},
};
use take_order::TakeOrderCodec;

const TOPIC: &str = "Herc20Hbit";

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent", poll_method = "poll")]
pub struct Orderbook {
    pub gossipsub: Gossipsub,
    take_order: RequestResponse<TakeOrderCodec>,
    #[behaviour(ignore)]
    events: VecDeque<BehaviourOutEvent>,
    #[behaviour(ignore)]
    orders: HashMap<OrderId, Order>,
    #[behaviour(ignore)]
    trading_pairs: HashSet<TradingPairTopic>,
    #[behaviour(ignore)]
    identities: HashMap<OrderId, (crate::bitcoin::Address, identity::Ethereum)>,
    #[behaviour(ignore)]
    pub(crate) peer_id: PeerId,
}

impl Orderbook {
    pub fn new(peer_id: PeerId) -> Orderbook {
        // To content-address message, we can take the hash of message and use it as an
        // ID.
        let message_id_fn = |message: &gossipsub::GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId(s.finish().to_string())
        };
        let gossipsub_config = gossipsub::GossipsubConfigBuilder::new()
            .heartbeat_interval(Duration::from_secs(1))
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the
            // same content will be propagated.
            .build();

        let mut config = RequestResponseConfig::default();
        config.set_request_timeout(Duration::from_secs(5 * 60));

        let mut orderbook = Orderbook {
            peer_id: peer_id.clone(),
            gossipsub: Gossipsub::new(peer_id, gossipsub_config),
            take_order: RequestResponse::new(
                TakeOrderCodec::default(),
                vec![(TakeOrderProtocol, ProtocolSupport::Full)],
                config,
            ),
            trading_pairs: HashSet::new(),
            orders: HashMap::new(),
            identities: HashMap::new(),
            events: VecDeque::new(),
        };

        orderbook.gossipsub.subscribe(DefaultMakerTopic::to_topic());

        orderbook.gossipsub.subscribe(Topic::new(TOPIC.to_string()));

        orderbook
    }

    fn confirm(&mut self, _peer: PeerId, channel: ResponseChannel<Response>) {
        tracing::info!("confirming order");
        self.take_order
            .send_response(channel, Response::Confirmation);
        self.events.push_back(BehaviourOutEvent::TakeOrderRequest)
    }

    fn deny(&mut self, peer: PeerId, channel: ResponseChannel<Response>) {
        tracing::info!("aborting announce protocol with {}", peer);

        self.events.push_back(BehaviourOutEvent::Failed(peer));
        self.take_order.send_response(channel, Response::Error);
    }

    fn can_order_be_taken(&self, order_id: &OrderId) -> bool {
        self.orders.contains_key(order_id)
    }

    pub fn make(
        &mut self,
        order: Order,
        refund_identity: crate::bitcoin::Address,
        redeem_identity: identity::Ethereum,
    ) -> anyhow::Result<OrderId> {
        self.gossipsub.publish(
            &order.topic(&self.peer_id),
            bincode::serialize(&Message::CreateOrder(order.clone())).unwrap(),
        );
        tracing::info!("order published");
        self.orders.insert(order.id, order.clone());
        self.identities
            .insert(order.id, (refund_identity, redeem_identity));
        Ok(order.id)
    }

    pub fn old_take(
        &mut self,
        order_id: OrderId,
    ) -> anyhow::Result<(Order, crate::bitcoin::Address, identity::Ethereum)> {
        let identities = match self.identities.remove(&order_id) {
            Some(identities) => (identities.0, identities.1),
            None => {
                return Err(anyhow::Error::from(
                    OrderbookError::IdentitiesForOrderNotFound(order_id),
                ))
            }
        };
        let order = match self.orders.remove(&order_id) {
            Some(order) => order,
            None => return Err(anyhow::Error::from(OrderbookError::OrderNotFound(order_id))),
        };
        Ok((order, identities.0, identities.1))
    }

    pub fn take(&mut self, peer_id: &PeerId, order_id: OrderId) -> RequestId {
        self.take_order.send_request(peer_id, order_id)
    }

    pub fn take_with_identities(
        &mut self,
        order_id: OrderId,
        refund_identity: crate::bitcoin::Address,
        redeem_identity: identity::Ethereum,
    ) -> anyhow::Result<Order> {
        self.identities
            .insert(order_id, (refund_identity, redeem_identity));
        let order = match self.orders.remove(&order_id) {
            Some(order) => order,
            None => return Err(anyhow::Error::from(OrderbookError::OrderNotFound(order_id))),
        };
        Ok(order)
    }

    pub fn get_orders(&self) -> Vec<Order> {
        #[allow(clippy::map_clone)]
        self.orders.values().map(|order| order.clone()).collect()
    }

    pub fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        if let Some(order) = self.orders.get(order_id) {
            Some(order.clone())
        } else {
            None
        }
    }

    pub fn get_trading_pairs(&self) -> Vec<TradingPairTopic> {
        let mut topics = vec![];
        for topic in self.trading_pairs.iter() {
            topics.push(topic.clone());
        }
        topics
    }

    pub fn add_trading_pair(&mut self, topic: &TradingPairTopic) {
        self.trading_pairs.insert(topic.clone());
    }

    pub fn subscribe(&mut self, peer: PeerId, trading_pair: TradingPair) -> anyhow::Result<()> {
        let topic = TradingPairTopic::new(peer, trading_pair);
        self.trading_pairs.insert(topic.clone());
        self.gossipsub.subscribe(topic.to_topic());
        Ok(())
    }

    pub fn unsubscribe(&mut self, peer: PeerId, trading_pair: TradingPair) -> anyhow::Result<()> {
        let topic = TradingPairTopic::new(peer, trading_pair);
        self.trading_pairs.remove(&topic);
        self.gossipsub.unsubscribe(topic.to_topic());
        Ok(())
    }

    // Ideally this step should be executed automatically before making an order.
    // Unfortunately a brief delay is required to allow peers to acknowledge and
    // subscribe to the announced trading pair before publishing the order
    pub fn announce_trading_pair(&mut self, trading_pair: TradingPair) {
        let topic = TradingPairTopic::new(self.peer_id.clone(), trading_pair).to_topic();
        self.gossipsub.subscribe(topic.clone());
        self.gossipsub.publish(
            &topic,
            bincode::serialize(&Message::TradingPair(trading_pair)).unwrap(),
        );
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

#[derive(thiserror::Error, Debug)]
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

impl MakerId {
    /// Returns a clone of the inner peer id.
    pub fn peer_id(&self) -> PeerId {
        self.0.clone()
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
    pub maker_addr: Multiaddr,
    pub buy: u64,
    pub sell: asset::Erc20,
    pub absolute_expiry: u32,
}

pub struct NewOrder {
    pub buy: asset::Bitcoin,
    pub sell: asset::Erc20,
    pub absolute_expiry: u32,
    pub maker_addr: Multiaddr,
}

impl Order {
    pub fn new(peer_id: PeerId, new_order: NewOrder) -> Self {
        Order {
            id: Uuid::new_v4(),
            maker: MakerId(peer_id),
            maker_addr: new_order.maker_addr,
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

pub struct DefaultMakerTopic;

impl DefaultMakerTopic {
    pub fn to_topic() -> Topic {
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

#[derive(Debug, PartialEq)]
pub enum BehaviourOutEvent {
    TakeOrderRequest,
    TakeOrderConfirmation,
    Failed(PeerId),
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for Orderbook {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message(peer_id, _message_id, message) = event {
            let decoded: Message = bincode::deserialize(&message.data[..]).unwrap();
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
                peer,
                message:
                    RequestResponseMessage::Request {
                        request: order_id,
                        channel,
                    },
            } => {
                if self.can_order_be_taken(&order_id) {
                    self.confirm(peer, channel);
                } else {
                    self.deny(peer, channel);
                }
            }
            RequestResponseEvent::Message {
                peer: _peer,
                message:
                    RequestResponseMessage::Response {
                        request_id: _request_id,
                        response: _response,
                    },
            } => {
                tracing::info!("received order confirmation");
                self.events
                    .push_back(BehaviourOutEvent::TakeOrderConfirmation);
            }
            RequestResponseEvent::OutboundFailure {
                peer: _peer,
                request_id: _request_id,
                error,
            } => {
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
        network::test::{await_events_or_timeout, connect, new_swarm},
    };
    use atty::{self, Stream};
    use libp2p::Swarm;
    use log::LevelFilter;
    use spectral::prelude::*;
    use std::str::FromStr;
    use tracing::{info, subscriber, Level};
    use tracing_log::LogTracer;
    use tracing_subscriber::FmtSubscriber;

    fn create_order(id: PeerId, maker_addr: Multiaddr) -> Order {
        Order::new(id, NewOrder {
            buy: asset::Bitcoin::from_sat(100),
            sell: Erc20 {
                token_contract: Default::default(),
                quantity: Erc20Quantity::max_value(),
            },
            absolute_expiry: 100,
            maker_addr,
        })
    }

    fn refund_redeem_identities() -> (crate::bitcoin::Address, identity::Ethereum) {
        let refund = crate::bitcoin::Address::from_str("2MufZ6LLCqYTvZnCwzfAjSKn26Xcnr19PBE")
            .expect("failed to parse bitcoin address");

        let redeem = identity::Ethereum::random();

        (refund, redeem)
    }

    #[tokio::test]
    async fn take_order_request_confirmation() {
        let comit_level = LevelFilter::Debug;
        let upstream_level = LevelFilter::Info;
        init_tracing(comit_level, upstream_level).expect("failed to init tracing");

        // arrange

        let delay = Duration::from_secs(2);

        let (mut alice_swarm, _, _alice_peer_id) = new_swarm(Orderbook::new);
        let (mut bob_swarm, bob_addr, bob_peer_id) = new_swarm(Orderbook::new);
        connect(&mut alice_swarm, &mut bob_swarm).await;

        let order = create_order(bob_peer_id.clone(), bob_addr.clone());
        let (bob_refund_identity, bob_redeem_identity) = refund_redeem_identities();

        // act

        // Trigger subscription.
        poll_with_delay(&mut alice_swarm, delay).await;
        poll_with_delay(&mut bob_swarm, delay).await;

        let _ = bob_swarm
            .make(order.clone(), bob_refund_identity, bob_redeem_identity)
            .expect("order id should exist");

        // Trigger publish and receipt of order.
        poll_with_delay(&mut bob_swarm, delay).await;
        poll_with_delay(&mut alice_swarm, delay).await;

        let order = alice_swarm
            .get_orders()
            .first()
            .cloned()
            .expect("Alice has no orders");

        tracing::info!("alice received order");

        alice_swarm.take(&bob_peer_id, order.id);

        // Trigger request/response messages.
        poll_with_delay(&mut bob_swarm, delay).await;
        poll_with_delay(&mut alice_swarm, delay).await;

        let (alice_event, bob_event) =
            await_events_or_timeout(alice_swarm.next(), bob_swarm.next()).await;

        // assert

        assert_that(&alice_event).is_equal_to(BehaviourOutEvent::TakeOrderConfirmation);
        assert_that(&bob_event).is_equal_to(BehaviourOutEvent::TakeOrderRequest);
    }

    // Poll the swarm for some time, we don't expect any events though.
    async fn poll_with_delay(swarm: &mut Swarm<Orderbook>, delay: Duration) {
        while let Ok(event) = tokio::time::timeout(delay, swarm.next()).await {
            panic!("unexpected event emitted: {:?}", event)
        }
    }

    fn init_tracing(comit: LevelFilter, upstream: LevelFilter) -> anyhow::Result<()> {
        LogTracer::init_with_filter(upstream)?;

        let is_terminal = atty::is(Stream::Stdout);
        let subscriber = FmtSubscriber::builder()
            .with_max_level(level_from_level_filter(comit))
            .with_ansi(is_terminal)
            .finish();

        subscriber::set_global_default(subscriber)?;
        info!(
            "Initialized tracing within comit at level: {}, upstream: {}",
            comit, upstream
        );

        Ok(())
    }

    fn level_from_level_filter(level: LevelFilter) -> Level {
        match level {
            LevelFilter::Off => panic!("level is off"),
            LevelFilter::Error => Level::ERROR,
            LevelFilter::Warn => Level::WARN,
            LevelFilter::Info => Level::INFO,
            LevelFilter::Debug => Level::DEBUG,
            LevelFilter::Trace => Level::TRACE,
        }
    }
}
