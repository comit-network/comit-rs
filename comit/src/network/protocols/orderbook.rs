use libp2p::{
    gossipsub,
    gossipsub::{Gossipsub, GossipsubEvent, Topic},
    multiaddr::Multiaddr,
    NetworkBehaviour,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    time::Duration,
};
use uuid::Uuid;

use crate::{asset, identity};
use libp2p::{swarm::NetworkBehaviourEventProcess, PeerId};
use serde::de::Error;
use std::str::FromStr;

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
        Topic::new("HbitHerc20".to_string())
    }
}

pub type OrderId = Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct MakerId(PeerId);

impl MakerId {
    pub fn new(peer_id: PeerId) -> Self {
        MakerId(peer_id)
    }
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

#[derive(Debug)]
pub struct OrderWithIdentities {
    pub id: OrderId,
    pub maker_id: Vec<u8>,
    pub maker_addr: Multiaddr,
    pub buy: u64,
    pub sell: asset::Erc20,
    pub absolute_expiry: u32,
}

#[derive(Debug)]
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
    pub fn order_id(&self) -> OrderId {
        self.id
    }
    pub fn topic(&self, peer: &PeerId) -> Topic {
        TradingPair {
            buy: SwapType::Hbit,
            sell: SwapType::Herc20,
        }
        .to_topic(peer)
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourOutEvent")]
pub struct Orderbook {
    pub gossipsub: Gossipsub,
    #[behaviour(ignore)]
    orders: HashMap<OrderId, Order>,
    #[behaviour(ignore)]
    trading_pairs: HashSet<TradingPairTopic>,
    #[behaviour(ignore)]
    identities: HashMap<OrderId, (crate::bitcoin::Address, identity::Ethereum)>,
    #[behaviour(ignore)]
    pub(crate) peer_id: PeerId,
}

impl fmt::Debug for Orderbook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicOrderbook")
            .field("peer_id", &self.peer_id)
            .field("orders", &self.orders)
            .finish()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BehaviourOutEvent;

impl NetworkBehaviourEventProcess<GossipsubEvent> for Orderbook {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message(peer_id, _message_id, message) = event {
            let decoded: Message = bincode::deserialize(&message.data[..]).unwrap();
            match decoded {
                Message::CreateOrder(order) => {
                    tracing::info!("create order message received");
                    self.orders.insert(order.order_id(), order);
                }
                Message::DeleteOrder(order_id) => {
                    self.orders.remove(&order_id);
                }
                Message::TradingPair(trading_pair) => {
                    tracing::info!("trading pair message received");
                    self.add_trading_pair(&TradingPairTopic::new(peer_id.clone(), trading_pair));
                    self.gossipsub
                        .subscribe(TradingPairTopic::new(peer_id, trading_pair).to_topic());
                }
            }
        }
    }
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
        let mut orderbook = Orderbook {
            peer_id: peer_id.clone(),
            gossipsub: Gossipsub::new(peer_id, gossipsub_config),
            trading_pairs: HashSet::new(),
            orders: HashMap::new(),
            identities: HashMap::new(),
        };

        orderbook.gossipsub.subscribe(DefaultMakerTopic::to_topic());

        orderbook
            .gossipsub
            .subscribe(Topic::new("HbitHerc20".to_string()));

        orderbook
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

    pub fn take(
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
        self.orders.values().cloned().collect()
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
}
