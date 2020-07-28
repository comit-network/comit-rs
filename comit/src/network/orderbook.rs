use crate::{Order, OrderId, BTC_DAI};
use libp2p::{
    gossipsub,
    gossipsub::{Gossipsub, GossipsubEvent, Topic},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour, PeerId,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fmt,
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
    time::Duration,
};

/// The Orderbook libp2p network behaviour.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct Orderbook {
    gossipsub: Gossipsub,
    #[behaviour(ignore)]
    orders: HashMap<OrderId, Order>,
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

        let mut orderbook = Orderbook {
            peer_id,
            gossipsub,
            orders: HashMap::new(),
        };

        // Since we only support a single trading pair topic just subscribe to it now.
        orderbook
            .gossipsub
            .subscribe(Topic::new(BTC_DAI.to_string()));

        orderbook
    }

    // Currently we only implement publishing a create order message.
    // TODO: Implement API to publish a delete order message.

    /// Publish a limit order to the gossipsub network.
    // Called by Bob i.e., the maker.
    pub fn publish(&mut self, order: Order) -> anyhow::Result<OrderId> {
        let ser = bincode::serialize(&Message::CreateOrder(order.clone()))?;
        let topic = order.tp().to_topic();
        self.gossipsub.publish(&topic, ser);

        let id = order.id;
        self.orders.insert(id, order);

        Ok(id)
    }

    /// Get a list of all orders known to this node.
    pub fn get_orders(&self) -> Vec<Order> {
        self.orders.values().cloned().collect()
    }

    /// Get the order matching `id` if known to this node.
    pub fn get_order(&self, id: &OrderId) -> Option<Order> {
        self.orders.get(id).cloned()
    }
}

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
#[error("order {0} not found in orderbook")]
pub struct OrderNotFound(OrderId);

/// MakerId is a PeerId wrapper so we control serialization/deserialization.
#[derive(Debug, Clone, PartialEq)]
pub struct MakerId(PeerId);

impl MakerId {
    pub fn new(id: PeerId) -> Self {
        MakerId(id)
    }
}

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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Message {
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

impl NetworkBehaviourEventProcess<GossipsubEvent> for Orderbook {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message(_peer_id, _message_id, message) = event {
            let decoded: Message = match bincode::deserialize(&message.data[..]) {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::warn!("deserialization of gossipsub message failed: {}", e);
                    return;
                }
            };

            match decoded {
                Message::CreateOrder(order) => {
                    // This implies new orders remove old orders from
                    // the orderbook. This means nodes can spoof the
                    // network using previously seen order ids in
                    // order to override orders.
                    self.orders.insert(order.id, order);
                }
                Message::DeleteOrder(order_id) => {
                    // Same consideration here, nodes can cause orders
                    // they did not create to be removed by spoofing
                    // the network with a previously seen order id.
                    self.orders.remove(&order_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

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
