use crate::ethereum::U256;
use bigdecimal::BigDecimal;
use lazy_static::lazy_static;
use num::{
    bigint::{ParseBigIntError, Sign},
    BigInt, BigUint, Zero,
};
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{fmt, ops::Div, str::FromStr};

lazy_static! {
    static ref WEI_IN_ETHER_U128: u128 = (10u128).pow(18);
    static ref WEI_IN_ETHER_BIGUINT: BigUint = BigUint::from(*WEI_IN_ETHER_U128);
    static ref WEI_IN_ETHER_BIGDEC: BigDecimal = BigDecimal::from((
        BigInt::from_biguint(Sign::Plus, WEI_IN_ETHER_BIGUINT.clone()),
        0
    ));
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Ether(BigUint);

impl Ether {
    pub fn max_value() -> Self {
        Self(BigUint::from(std::u64::MAX) * 4u64)
    }

    pub fn to_u256(&self) -> U256 {
        let buf = self.0.to_bytes_be();
        U256::from_big_endian(&buf)
    }

    pub fn zero() -> Self {
        Self(BigUint::zero())
    }
}

impl fmt::Display for Ether {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let big_int = BigInt::from_biguint(Sign::Plus, self.clone().0);
        let dec = BigDecimal::from((big_int, 0));
        let ether = dec.div(WEI_IN_ETHER_BIGDEC.clone());
        write!(f, "{} ETH", ether)
    }
}

pub trait FromWei<W> {
    fn from_wei(wei: W) -> Self;
}

pub trait TryFromWei<W>
where
    Self: std::marker::Sized,
{
    type Err;
    fn try_from_wei(wei: W) -> Result<Self, Self::Err>;
}

macro_rules! impl_from_wei_primitive {
    ($primitive:ty) => {
        impl FromWei<$primitive> for Ether {
            fn from_wei(w: $primitive) -> Self {
                Ether(BigUint::from(w))
            }
        }
    };
}

impl_from_wei_primitive!(u8);
impl_from_wei_primitive!(u16);
impl_from_wei_primitive!(u32);
impl_from_wei_primitive!(u64);
impl_from_wei_primitive!(u128);

impl FromWei<U256> for Ether {
    fn from_wei(wei: U256) -> Self {
        let mut buf = [0u8; 32];
        wei.to_big_endian(&mut buf);
        Ether(BigUint::from_bytes_be(&buf))
    }
}

impl FromWei<BigUint> for Ether {
    fn from_wei(wei: BigUint) -> Self {
        Ether(wei)
    }
}

impl TryFromWei<&str> for Ether {
    type Err = ParseBigIntError;

    fn try_from_wei(string: &str) -> Result<Ether, Self::Err> {
        let uint = BigUint::from_str(string)?;
        Ok(Self::from_wei(uint))
    }
}

impl<'de> Deserialize<'de> for Ether {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = Ether;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("A string representing a wei quantity")
            }

            fn visit_str<E>(self, v: &str) -> Result<Ether, E>
            where
                E: de::Error,
            {
                let wei = BigUint::from_str(v).map_err(E::custom)?;
                Ok(Ether(wei))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for Ether {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_one_thousand_in_u256_equals_one_thousand_u32() {
        let u256 = U256::from(1000);
        let u256 = Ether::from_wei(u256);
        let u32 = Ether::from_wei(1000u32);

        assert_eq!(u256, u32)
    }

    #[test]
    fn from_one_thousand_in_u32_converts_to_u256() {
        let ether = Ether::from_wei(1000u32);

        let u256 = U256::from(1000);

        assert_eq!(ether.to_u256(), u256)
    }

    #[test]
    fn given_9000_exa_wei_display_in_ether() {
        assert_eq!(
            Ether::from_wei(9000 * *WEI_IN_ETHER_U128).to_string(),
            "9000 ETH"
        );
    }

    #[test]
    fn given_1_peta_wei_display_in_ether() {
        assert_eq!(
            Ether::from_wei(1_000_000_000_000_000u128).to_string(),
            "0.001 ETH"
        );
    }

    #[test]
    fn try_from_wei_dec_str_equals_from_wei_u128() {
        let from_str = Ether::try_from_wei("9001000000000000000000").unwrap();
        let from_u128 = Ether::from_wei(9_001_000_000_000_000_000_000u128);

        assert_eq!(from_str, from_u128)
    }

    #[test]
    fn serialize() {
        let ether = Ether::from_wei(*WEI_IN_ETHER_U128);
        let ether_str = serde_json::to_string(&ether).unwrap();
        assert_eq!(ether_str, "\"1000000000000000000\"");
    }

    #[test]
    fn deserialize_ether_quantity() {
        let ether_str = "\"1000000000000000000\"";
        let ether = serde_json::from_str::<Ether>(ether_str).unwrap();
        assert_eq!(ether, Ether::from_wei(*WEI_IN_ETHER_U128));
    }
}
