use crate::{
    asset::ethereum::{Error, FromWei, TryFromWei},
    ethereum::U256,
};
use conquer_once::Lazy;
use num::{pow::Pow, BigUint, Integer, Num, Zero};
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{fmt, str::FromStr};

static WEI_IN_ETHER_U128: Lazy<u128> = Lazy::new(|| (10u128).pow(18));
static WEI_IN_ETHER_BIGUINT: Lazy<BigUint> = Lazy::new(|| BigUint::from(*WEI_IN_ETHER_U128));

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Ether(BigUint);

impl Ether {
    pub fn zero() -> Self {
        Self(BigUint::zero())
    }

    pub fn max_value() -> Self {
        Self(BigUint::from(2u8).pow(256u32) - 1u8)
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

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes_le()
    }
}

impl fmt::Display for Ether {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let (ether, rem) = self.0.div_rem(&WEI_IN_ETHER_BIGUINT);

        if rem.is_zero() {
            write!(f, "{} ETH", ether)
        } else {
            // format number as base 10
            let rem = rem.to_str_radix(10);

            // prefix with 0 in the front until we have 18 chars
            let rem = format!("{:0>18}", rem);

            // trim unnecessary 0s from the back
            let rem = rem.trim_end_matches('0');

            write!(f, "{}.{} ETH", ether, rem)
        }
    }
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

impl TryFromWei<BigUint> for Ether {
    fn try_from_wei(wei: BigUint) -> Result<Self, Error> {
        if wei > Self::max_value().0 {
            Err(Error::Overflow)
        } else {
            Ok(Self(wei))
        }
    }
}

impl TryFromWei<&str> for Ether {
    fn try_from_wei(string: &str) -> Result<Ether, Error> {
        let uint = BigUint::from_str(string)?;
        Ok(Self(uint))
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
                let quantity = Ether::try_from_wei(wei).map_err(E::custom)?;
                Ok(quantity)
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
        let u256 = U256::from(1_000);
        let u256 = Ether::from_wei(u256);
        let u32 = Ether::from_wei(1_000u32);

        assert_eq!(u256, u32)
    }

    #[test]
    fn from_one_thousand_in_u32_converts_to_u256() {
        let ether = Ether::from_wei(1_000u32);
        let u256 = U256::from(1_000);

        assert_eq!(ether.to_u256(), u256)
    }

    #[test]
    fn given_9000_exa_wei_display_in_ether() {
        assert_eq!(
            Ether::from_wei(9_000 * *WEI_IN_ETHER_U128).to_string(),
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
    fn given_some_weird_wei_number_formats_correctly_as_eth() {
        assert_eq!(
            Ether::from_wei(1_003_564_412_000_000_000u128).to_string(),
            "1.003564412 ETH"
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
    fn deserialize() {
        let ether_str = "\"1000000000000000000\"";
        let ether = serde_json::from_str::<Ether>(ether_str).unwrap();
        assert_eq!(ether, Ether::from_wei(*WEI_IN_ETHER_U128));
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
        let quantity = Ether::try_from_wei(wei);
        assert_eq!(quantity, Err(Error::Overflow))
    }

    #[test]
    fn given_too_big_string_when_deserializing_return_overflow_error() {
        let quantity_str =
            "\"115792089237316195423570985008687907853269984665640564039457584007913129639936\""; // This is Ether::max_value() + 1
        let res = serde_json::from_str::<Ether>(quantity_str);
        assert!(res.is_err())
    }

    #[test]
    fn to_dec() {
        let ether = Ether::from_wei(12_345u32);
        assert_eq!(ether.to_wei_dec(), "12345".to_string())
    }

    #[test]
    fn given_str_of_wei_in_dec_format_instantiate_ether() {
        let ether = Ether::from_wei_dec_str("12345").unwrap();
        assert_eq!(ether, Ether::from_wei(12_345u32))
    }

    #[test]
    fn given_str_above_u256_max_in_dec_format_return_overflow() {
        let res = Ether::from_wei_dec_str(
            "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        ); // This is Ether::max_value() + 1
        assert_eq!(res, Err(Error::Overflow))
    }
}
