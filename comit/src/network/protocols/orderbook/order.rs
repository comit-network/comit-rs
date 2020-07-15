use crate::{
    asset, ledger,
    network::protocols::orderbook::{MakerId, SwapType, TradingPair},
};
use libp2p::{gossipsub::Topic, PeerId};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderId(Uuid);

impl OrderId {
    pub fn random() -> OrderId {
        OrderId(Uuid::new_v4())
    }
}

impl Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for OrderId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::from_str(s)?;
        Ok(OrderId(uuid))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub id: OrderId,
    pub maker: MakerId,
    pub buy: u64,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub sell: asset::Erc20,
    pub ethereum_ledger: ledger::Ethereum,
    pub absolute_expiry: u32,
}

#[derive(Debug)]
pub struct NewOrder {
    pub buy: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub sell: asset::Erc20,
    pub ethereum_ledger: ledger::Ethereum,
    pub absolute_expiry: u32,
}

impl Order {
    pub fn new(peer_id: PeerId, new_order: NewOrder) -> Self {
        Order {
            id: OrderId::random(),
            maker: MakerId(peer_id),
            buy: new_order.buy.as_sat(),
            bitcoin_ledger: new_order.bitcoin_ledger,
            sell: new_order.sell,
            ethereum_ledger: new_order.ethereum_ledger,
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

impl NewOrder {
    pub fn assert_valid_ledger_pair(&self) -> anyhow::Result<()> {
        let a = self.bitcoin_ledger;
        let b = self.ethereum_ledger;

        if ledger::is_valid_ledger_pair(a, b) {
            return Ok(());
        }
        Err(anyhow::anyhow!("invalid ledger pair {}/{}", a, b))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn order_id_serialization_roundtrip() {
        // TODO: Implement order_id_serialization_roundtrip()
    }

    #[test]
    fn order_id_serialization_stability() {
        // TODO: Implement order_id_serialization_stability()
    }
}
