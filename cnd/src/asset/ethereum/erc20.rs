use crate::{
    asset::ethereum::{Error, FromWei, TryFromWei},
    ethereum::{Address, U256},
};
use num::{BigUint, Num, Zero};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Erc20Quantity(BigUint);

impl Erc20Quantity {
    pub fn zero() -> Self {
        Self(BigUint::zero())
    }

    pub fn max_value() -> Self {
        Self(BigUint::from(std::u64::MAX) * 4u64)
    }

    pub fn to_wei_dec(&self) -> String {
        self.0.to_str_radix(10)
    }

    pub fn from_wei_dec_str(str: &str) -> Result<Self, Error> {
        let int = BigUint::from_str_radix(str, 10)?;
        Ok(Self::try_from_wei(int)?)
    }

    pub fn to_u256(&self) -> U256 {
        let buf = self.0.to_bytes_be();
        U256::from_big_endian(&buf)
    }
}

impl FromWei<U256> for Erc20Quantity {
    fn from_wei(wei: U256) -> Self {
        let mut buf = [0u8; 32];
        wei.to_big_endian(&mut buf);
        Self(BigUint::from_bytes_be(&buf))
    }
}

impl fmt::Display for Erc20Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! impl_from_wei_primitive {
    ($primitive:ty) => {
        impl FromWei<$primitive> for Erc20Quantity {
            fn from_wei(w: $primitive) -> Self {
                Erc20Quantity(BigUint::from(w))
            }
        }
    };
}

impl_from_wei_primitive!(u8);
impl_from_wei_primitive!(u16);
impl_from_wei_primitive!(u32);
impl_from_wei_primitive!(u64);
impl_from_wei_primitive!(u128);

impl TryFromWei<BigUint> for Erc20Quantity {
    fn try_from_wei(wei: BigUint) -> Result<Self, Error> {
        if wei > Self::max_value().0 {
            Err(Error::Overflow)
        } else {
            Ok(Self(wei))
        }
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
                let wei = BigUint::from_str(v).map_err(E::custom)?;
                let quantity = Erc20Quantity::try_from_wei(wei).map_err(E::custom)?;
                Ok(quantity)
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

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
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
    fn display() {
        let quantity = Erc20Quantity::from_wei(123_456_789u64);
        assert_eq!(quantity.to_string(), "123456789".to_string());
    }

    #[test]
    fn serialize() {
        let quantity = Erc20Quantity::from_wei(123_456_789u64);
        let quantity_str = serde_json::to_string(&quantity).unwrap();
        assert_eq!(quantity_str, "\"123456789\"");
    }

    #[test]
    fn deserialize() {
        let quantity_str = "\"123456789\"";
        let quantity = serde_json::from_str::<Erc20Quantity>(quantity_str).unwrap();
        assert_eq!(quantity, Erc20Quantity::from_wei(123_456_789u64));
    }

    #[test]
    fn given_too_big_biguint_return_overflow_error() {
        let wei = BigUint::from_slice(&[
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX,
            std::u32::MAX, // 9th u32, should make it over u256
        ]);
        let quantity = Erc20Quantity::try_from_wei(wei);
        assert_eq!(quantity, Err(Error::Overflow))
    }

    #[test]
    fn given_too_big_string_when_deserializing_return_overflow_error() {
        let quantity_str = "\"73786976294838206461\""; // This is u256::MAX + 1
        let res = serde_json::from_str::<Erc20Quantity>(quantity_str);
        assert!(res.is_err())
    }

    #[test]
    fn to_dec() {
        let quantity = Erc20Quantity::from_wei(12345u32);
        assert_eq!(quantity.to_wei_dec(), "12345".to_string())
    }

    #[test]
    fn given_str_of_wei_in_dec_format_instantiate_ether() {
        let quantity = Erc20Quantity::from_wei_dec_str("12345").unwrap();
        assert_eq!(quantity, Erc20Quantity::from_wei(12345u32))
    }

    #[test]
    fn given_str_above_u256_max_in_dec_format_return_overflow() {
        let res = Erc20Quantity::from_wei_dec_str("73786976294838206461"); // This is u256::MAX + 1
        assert_eq!(res, Err(Error::Overflow))
    }
}
