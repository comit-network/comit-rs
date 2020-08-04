use crate::{asset, asset::Erc20Quantity, identity, ledger, network::orderbook::MakerId};
use num::BigUint;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

/// An order, created by a maker (Bob) and shared with the network via
/// gossipsub.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub id: OrderId,
    pub maker: MakerId,
    pub position: Position,
    pub price: Rate,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub bitcoin_absolute_expiry: u32,
    #[serde(with = "asset::bitcoin::sats_as_string")]
    pub bitcoin_quantity: asset::Bitcoin,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    pub ethereum_absolute_expiry: u32,
}

// We explicitly only support BTC/DAI.
impl Order {
    pub fn ethereum_amount(&self, amount: asset::Bitcoin) -> Erc20Quantity {
        Erc20Quantity::from(BigUint::from(amount.as_sat()) * (BigUint::from(self.price.0)))
    }
}

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
/// The amount of DAI in wei for one satoshi of BTC
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate(pub u64);

impl Serialize for Rate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let precision = 10;
        let base: u64 = 10;
        let remainder = self.0 % base.pow(precision);
        let integer = (self.0 - remainder) / base.pow(precision);
        let string = format!("{}.{}", integer, remainder);

        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Rate {
    /// Decimal precision of 10 is selected because it is the largest possible
    /// precision for BTC to DAI conversion rate.
    /// Ethereum supports a precision of 18 and Bitcoin supports a precision of
    /// 8 Therefore 10 is the maximum allowable precision for BTC to DAI
    /// conversion
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let precision = 10;
        let string = String::deserialize(deserializer)?;
        let v: Vec<&str> = string.as_str().split('.').collect();

        let integer = *v.first().unwrap();
        let decimals = *v.last().unwrap();
        if decimals.len() > precision {
            return Err(Error::custom(format!(
                "BTC to DAI rate does not support a decimal precision of {}, expected {}",
                decimals.len(),
                precision
            )));
        }
        let trailing_zeros = precision - decimals.len();

        let zero_vec = vec!['0'; trailing_zeros];
        let zeros: String = zero_vec.into_iter().collect();

        let result = format!("{}{}{}", integer, decimals, &zeros);

        let rate = u64::from_str(&result).unwrap();

        Ok(Rate(rate))
    }
}

#[cfg(test)]
pub fn meaningless_test_order(maker: MakerId) -> Order {
    Order {
        id: meaningless_test_order_id(),
        maker,
        position: Position::Sell,
        bitcoin_quantity: asset::Bitcoin::meaningless_test_value(),
        bitcoin_ledger: ledger::Bitcoin::Regtest,
        bitcoin_absolute_expiry: meaningless_expiry_value(),
        price: Rate(9000),
        token_contract: Default::default(),
        ethereum_ledger: ledger::Ethereum::default(),
        ethereum_absolute_expiry: meaningless_expiry_value(),
    }
}

#[cfg(test)]
fn meaningless_expiry_value() -> u32 {
    100
}

#[cfg(test)]
fn meaningless_test_order_id() -> OrderId {
    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    OrderId(uuid)
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
    fn rate_deserialization_stability() {
        let s = "\"11033.4600000000\"";
        let got: Rate = serde_json::from_str(&s).expect("failed to parse rate");
        let want = Rate(110_334_600_000_000);

        assert_that(&want).is_equal_to(got);
    }

    #[test]
    fn rate_deserialization_stability_2() {
        let s = "\"11033.46\"";
        let got: Rate = serde_json::from_str(&s).expect("failed to parse rate");
        let want = Rate(110_334_600_000_000);

        assert_that(&want).is_equal_to(got);
    }

    #[test]
    fn rate_serialization_stability() {
        let want = "\"11033.4600000000\"".to_string();
        let rate = Rate(110_334_600_000_000);
        let got = serde_json::to_string(&rate).expect("failed to serialise rate");
        assert_that(&got).is_equal_to(want);
    }

    #[test]
    fn rate_serialization_fails_too_many_decimals() {
        let s = "\"11033.46\"";
        let got: Rate = serde_json::from_str(&s).expect("failed to parse rate");
        let want = Rate(110334600000000);

        assert_that(&want).is_equal_to(got);
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

    // #[test]
    // fn btc_dai_order_serialization_stability() {
    //     let given = "QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY".to_string();
    //     let peer_id = PeerId::from_str(&given).expect("failed to parse peer id");
    //     let maker_id = MakerId(peer_id);
    //
    //     let order = meaningless_test_order(maker_id);
    //
    //     let got = serde_json::to_string(&order).expect("failed to serialize
    // order");     let want =
    // r#"{"id":"936da01f-9abd-4d9d-80c7-02af85c822a8","maker":"
    // QmfUfpC2frwFvcDzpspnfZitHt5wct6n4kpG5jzgRdsxkY","position":"sell","rate":
    // 9000,"bitcoin_ledger":"regtest","bitcoin_absolute_expiry":100,"
    // bitcoin_amount":"1000","token_contract":"
    // 0x0000000000000000000000000000000000000000","ethereum_ledger":{"chain_id":
    // 1337},"ethereum_absolute_expiry":100}"#.to_string();     assert_that(&
    // got).is_equal_to(want); }

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
