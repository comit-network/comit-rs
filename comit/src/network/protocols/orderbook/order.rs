use crate::{
    asset,
    network::protocols::orderbook::{Quote, BTC_DAI},
};

use libp2p::gossipsub::Topic;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

/// An limit order, created to supply liquidity to the network and
/// shared with the network via gossipsub.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BtcDaiOrder {
    pub id: OrderId,
    pub position: Position, // Position of the order from the creators perspective.
    pub quote: Quote,       // Indirect quote i.e., one unit of BTC = quote units of DAI.
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub amount: asset::Bitcoin, // Orders are quoted in the base currency.
}

impl BtcDaiOrder {
    /// Create a new buy limit order.
    pub fn new_buy(quote: Quote, amount: asset::Bitcoin) -> Self {
        BtcDaiOrder::new(quote, amount, Position::Buy)
    }

    /// Create a new sell limit order.
    pub fn new_sell(quote: Quote, amount: asset::Bitcoin) -> Self {
        BtcDaiOrder::new(quote, amount, Position::Sell)
    }

    fn new(quote: Quote, amount: asset::Bitcoin, position: Position) -> Self {
        BtcDaiOrder {
            id: OrderId::random(),
            position,
            quote,
            amount,
        }
    }

    /// Convert this order to the Topic used to publish it.
    pub fn to_topic(&self) -> Topic {
        Topic::new(BTC_DAI.to_string())
    }

    /// Converts forex terminology (quote/amount) to COMIT terminology.
    pub fn value(&self) -> (asset::Bitcoin, asset::Erc20Quantity) {
        unimplemented!()
    }

    #[cfg(test)]
    pub fn meaningless_test_value() -> Self {
        BtcDaiOrder {
            id: OrderId::random(),
            position: Position::Sell,
            quote: Quote::from_float(9123.45).expect("failed to construct quote"),
            amount: asset::Bitcoin::from_sat(1000),
        }
    }
}

/// The identifier used for orders.
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderId(Uuid);

impl OrderId {
    /// Create a random identifier, this should be globally unique.
    pub fn random() -> OrderId {
        OrderId(Uuid::new_v4())
    }

    #[cfg(test)]
    pub fn meaningless_test_value() -> OrderId {
        let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
        OrderId(uuid)
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

/// The position of the maker for this order. A BTC/DAI buy order,
/// also described as an order that buys the trading pair BTC/DAI,
/// means that the maker buys the base currency (in this case BTC) in
/// return for DAI. A sell order means that the maker sells BTC and
/// receives DAI.
///
/// Please note: we do not set the base currency to 1 and use rate
/// (i.e., quote currency) and amount as is commonly done in Forex
/// trading. We use the amounts of each currency to determine the
/// rate.
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
        let order_id = OrderId::meaningless_test_value();
        let json = serde_json::to_string(&order_id).expect("failed to serialize order id");
        let rinsed: OrderId = serde_json::from_str(&json).expect("failed to deserialize order id");
        assert_that(&rinsed).is_equal_to(&order_id);
    }

    #[test]
    fn order_id_serialization_stability() {
        let s = "936DA01F9ABD4d9d80C702AF85C822A8";
        let uuid = Uuid::from_str(s).expect("failed to parse uuid string");
        let order_id = OrderId(uuid);

        let want = "\"936da01f-9abd-4d9d-80c7-02af85c822a8\"".to_string();
        let got = serde_json::to_string(&order_id).expect("failed to serialize order id");
        assert_that(&got).is_equal_to(want);
    }

    #[test]
    fn position_serialization_roundtrip() {
        let pos = Position::Buy;
        let json = serde_json::to_string(&pos).expect("failed to serialize position");
        let rinsed: Position = serde_json::from_str(&json).expect("failed to deserialize position");

        assert_that(&rinsed).is_equal_to(&pos);
    }

    #[test]
    fn position_buy_serialization_stability() {
        let pos = Position::Buy;
        let s = serde_json::to_string(&pos).expect("failed to serialize position");
        assert_that(&s).is_equal_to(r#""buy""#.to_string());
    }

    #[test]
    fn position_sell_serialization_stability() {
        let pos = Position::Sell;
        let s = serde_json::to_string(&pos).expect("failed to serialize position");
        assert_that(&s).is_equal_to(r#""sell""#.to_string());
    }

    #[test]
    fn btc_dai_order_serialization_stability() {
        // TODO: implement btc_dai_order_serialization_stability()
    }
}
