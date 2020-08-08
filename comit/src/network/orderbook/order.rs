use crate::{asset, asset::ethereum::TryFromWei, identity, ledger};
use libp2p::PeerId;
use num::BigUint;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

/// An order, created by a maker (Bob) and shared with the network via
/// gossipsub.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Order {
    pub id: OrderId,
    pub maker: PeerId,
    pub position: Position,
    pub price: BtcDaiRate,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub bitcoin_absolute_expiry: u32,
    pub quantity: asset::Bitcoin,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    pub ethereum_absolute_expiry: u32,
}

impl Order {
    /// Calculates the ethereum quantity that corresponds to a partial take
    /// quantity in bitcoin
    pub fn ethereum_quantity(
        &self,
        bitcoin_quantity: asset::Bitcoin,
    ) -> anyhow::Result<asset::Erc20Quantity> {
        let wei = BigUint::from(bitcoin_quantity.as_sat()) * BigUint::from(self.price.0);
        Ok(asset::Erc20Quantity::try_from_wei(wei)?)
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

/// Represents a "form" that contains all data for creating a new order.
#[derive(Copy, Clone, Debug)]
pub struct NewOrder {
    pub position: Position,
    pub quantity: asset::Bitcoin,
    pub bitcoin_ledger: ledger::Bitcoin,
    pub bitcoin_absolute_expiry: u32,
    pub price: BtcDaiRate,
    pub token_contract: identity::Ethereum,
    pub ethereum_ledger: ledger::Ethereum,
    pub ethereum_absolute_expiry: u32,
}

impl NewOrder {
    // TODO: I think this should live in the controller an be asserted before we
    // construct the type
    pub fn assert_valid_ledger_pair(&self) -> anyhow::Result<()> {
        let a = self.bitcoin_ledger;
        let b = self.ethereum_ledger;

        if ledger::is_valid_ledger_pair(a, b) {
            return Ok(());
        }
        Err(anyhow::anyhow!("invalid ledger pair {}/{}", a, b))
    }
}

/// The position of the maker for this order. A BTC/DAI buy order,
/// also described as an order that buys the trading pair BTC/DAI,
/// means that the maker buys the base currency (in this case BTC) in
/// return for DAI. A sell order means that the maker sells BTC and
/// receives DAI.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Buy,
    Sell,
}
/// BTC to DAI conversion rate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BtcDaiRate(pub u64);

impl Serialize for BtcDaiRate {
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

impl<'de> Deserialize<'de> for BtcDaiRate {
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

        let rate =
            u64::from_str(result.as_str()).map_err(<D as Deserializer<'de>>::Error::custom)?;

        Ok(BtcDaiRate(rate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::*;

    #[test]
    fn btc_dai_rate_serialization_success() {
        let expected = "\"0.1234567891\"".to_string();
        let rate = BtcDaiRate(1234567891);
        let actual = serde_json::to_string(&rate).expect("failed to serialise rate");
        assert_that(&actual).is_equal_to(expected);
    }

    #[test]
    fn btc_dai_rate_deserialization_fail_too_many_decimals() {
        let expected = "\"0.12345678912\"".to_string();
        assert!(serde_json::from_str::<BtcDaiRate>(&expected).is_err());
    }
    #[test]
    fn btc_dai_rate_deserialization_success() {
        let expected = "\"0.1234567891\"".to_string();
        assert!(serde_json::from_str::<BtcDaiRate>(&expected).is_ok());
    }
}
