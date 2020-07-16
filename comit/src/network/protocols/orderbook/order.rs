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

#[cfg(test)]
fn meaningless_test_order_id() -> OrderId {
    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    OrderId(uuid)
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
    pub bitcoin_absolute_expiry: u32,
    pub ethereum_amount: asset::Erc20Quantity,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    pub ethereum_absolute_expiry: u32,
}

impl Order {
    pub fn tp(&self) -> TradingPair {
        TradingPair::BtcDai
    }
}

#[cfg(test)]
pub fn meaningless_test_order(maker: MakerId) -> Order {
    Order {
        id: meaningless_test_order_id(),
        maker,
        position: Position::Sell,
        bitcoin_amount: asset::Bitcoin::meaningless_test_value(),
        bitcoin_ledger: ledger::Bitcoin::Regtest,
        bitcoin_absolute_expiry: meaningless_expiry_value(),
        ethereum_amount: asset::Erc20Quantity::meaningless_test_value(),
        token_contract: Default::default(),
        ethereum_ledger: ledger::Ethereum::default(),
        ethereum_absolute_expiry: meaningless_expiry_value(),
    }
}

#[cfg(test)]
fn meaningless_expiry_value() -> u32 {
    100
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
    use libp2p::PeerId;
    use spectral::prelude::*;

    #[test]
    fn order_id_serialization_roundtrip() {
        let order_id = meaningless_test_order_id();
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
    fn btc_dai_order_serialization_stability() {
        let given = "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY".to_string();
        let peer_id = PeerId::from_str(&given).expect("failed to parse peer id");
        let maker_id = MakerId(peer_id);

        let order = Order {
            id: meaningless_test_order_id(),
            maker: maker_id,
            position: Position::Sell,
            bitcoin_amount: asset::Bitcoin::meaningless_test_value(),
            bitcoin_ledger: ledger::Bitcoin::Regtest,
            bitcoin_absolute_expiry: meaningless_expiry_value(),
            ethereum_amount: asset::Erc20Quantity::meaningless_test_value(),
            token_contract: Default::default(),
            ethereum_ledger: ledger::Ethereum::default(),
            ethereum_absolute_expiry: meaningless_expiry_value(),
        };

        let got = serde_json::to_string(&order).expect("failed to serialize order");
        let want = r#"{"id":"936da01f-9abd-4d9d-80c7-02af85c822a8","maker":"QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY","position":"sell","bitcoin_amount":"1000","bitcoin_ledger":"regtest","bitcoin_absolute_expiry":100,"ethereum_amount":"1000","token_contract":"0x0000000000000000000000000000000000000000","ethereum_ledger":{"chain_id":1337},"ethereum_absolute_expiry":100}"#.to_string();
        assert_that(&got).is_equal_to(want);
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
