use crate::{
    asset, identity, ledger,
    network::protocols::orderbook::{MakerId, TradingPair},
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
    pub position: Position,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub bitcoin_amount: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub ethereum_amount: asset::Erc20Quantity,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    // TODO: Add both expiries
    pub absolute_expiry: u32,
}

impl Order {
    pub fn tp(&self) -> TradingPair {
        TradingPair::BtcDai
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Buy,
    Sell,
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

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

    #[test]
    fn trade_serialization_roundtrip() {
        let trade = Position::Buy;
        let json = serde_json::to_string(&trade).expect("failed to serialize trade");
        let rinsed: Position = serde_json::from_str(&json).expect("failed to deserialize trade");

        assert_that(&rinsed).is_equal_to(&trade);
    }

    #[test]
    fn trade_buy_serialization_stability() {
        let trade = Position::Buy;
        let s = serde_json::to_string(&trade).expect("failed to serialize trade");
        assert_that(&s).is_equal_to(r#""buy""#.to_string());
    }

    #[test]
    fn trade_sell_serialization_stability() {
        let trade = Position::Sell;
        let s = serde_json::to_string(&trade).expect("failed to serialize trade");
        assert_that(&s).is_equal_to(r#""sell""#.to_string());
    }
}
