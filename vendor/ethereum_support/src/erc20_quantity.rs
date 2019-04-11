use crate::{
    u256_ext::{FromDecimalStr, ToBigInt},
    U256,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Quantity(pub U256);

impl fmt::Display for Erc20Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_bigint())
    }
}

impl<'de> Deserialize<'de> for Erc20Quantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = Erc20Quantity;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("A string representing an ERC20 quantity")
            }

            fn visit_str<E>(self, v: &str) -> Result<Erc20Quantity, E>
            where
                E: de::Error,
            {
                U256::from_decimal_str(v)
                    .map(Erc20Quantity)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for Erc20Quantity {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{}", self).as_str())
    }
}

impl From<Erc20Quantity> for U256 {
    fn from(quantity: Erc20Quantity) -> U256 {
        quantity.0
    }
}
