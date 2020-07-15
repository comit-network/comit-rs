use crate::{
    asset, ledger,
    network::protocols::orderbook::{MakerId, TradingPair, BTC_DAI},
};
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
    pub trade: Trade,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub btc: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub dai: asset::Erc20,
    pub ethereum_ledger: ledger::Ethereum,
    // TODO: Add both expiries
    pub absolute_expiry: u32,
}

impl Order {
    pub fn tp(&self) -> TradingPair {
        TradingPair::BtcDai
    }

    pub fn topic(&self) -> String {
        BTC_DAI.to_string()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Trade {
    Buy,
    Sell,
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

    #[test]
    fn btc_dai_order_serialization_roundtrip() {
        // TODO: Implement btc_dai_order_serialization_roundtrip()
    }

    #[test]
    fn btc_dai_order_serialization_stability() {
        // TODO: Implement btc_dai_order_serialization_stability()
    }
}
