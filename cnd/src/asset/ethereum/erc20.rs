use crate::{
    asset::ethereum::FromWei,
    ethereum::{
        u256_ext::{FromDecimalStr, ToBigInt},
        Address, U256,
    },
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Quantity(U256);

impl Erc20Quantity {
    pub fn zero() -> Self {
        Self(U256::zero())
    }

    pub fn to_u256(self) -> U256 {
        self.0
    }
}

impl FromWei<U256> for Erc20Quantity {
    fn from_wei(wei: U256) -> Self {
        Self(wei)
    }
}

impl fmt::Display for Erc20Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_bigint())
    }
}

macro_rules! impl_from_wei_primitive {
    ($primitive:ty) => {
        impl FromWei<$primitive> for Erc20Quantity {
            fn from_wei(w: $primitive) -> Self {
                Erc20Quantity(U256::from(w))
            }
        }
    };
}

impl_from_wei_primitive!(u8);
impl_from_wei_primitive!(u16);
impl_from_wei_primitive!(u32);
impl_from_wei_primitive!(u64);
impl_from_wei_primitive!(u128);

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
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20 {
    pub token_contract: Address,
    pub quantity: Erc20Quantity,
}

impl fmt::Display for Erc20 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.quantity)
    }
}

impl Erc20 {
    pub fn new(token_contract: Address, quantity: Erc20Quantity) -> Self {
        Erc20 {
            token_contract,
            quantity,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn from_one_thousand_in_u256_equals_one_thousand_u32() {
        let u256 = U256::from(1000);
        let u256 = Erc20Quantity::from_wei(u256);
        let u32 = Erc20Quantity::from_wei(1000u32);

        assert_eq!(u256, u32)
    }

    #[test]
    fn from_one_thousand_in_u32_converts_to_u256() {
        let quantity = Erc20Quantity::from_wei(1000u32);

        let u256 = U256::from(1000);

        assert_eq!(quantity.to_u256(), u256)
    }

    #[test]
    fn serialize() {
        let quantity = Erc20Quantity::from_wei(123456789u64);
        let quantity_str = serde_json::to_string(&quantity).unwrap();
        assert_eq!(quantity_str, "\"123456789\"");
    }

    #[test]
    fn deserialize() {
        let quantity_str = "\"123456789\"";
        let quantity = serde_json::from_str::<Erc20Quantity>(quantity_str).unwrap();
        assert_eq!(quantity, Erc20Quantity::from_wei(123456789u64));
    }
}
