use crate::ethereum::{
    u256_ext::{FromBigUInt, FromDecimalStr, ToBigDecimal, ToDecimalStr, ToFloat},
    U256,
};
use bigdecimal::{BigDecimal, ParseBigDecimalError};
use num::FromPrimitive;
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{f64, fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Ether(U256);

impl Ether {
    fn from_eth_bigdec(decimal: &BigDecimal) -> Ether {
        let (wei_bigint, _) = decimal.with_scale(18).as_bigint_and_exponent();
        let wei = U256::from_biguint(wei_bigint.to_biguint().unwrap());
        Ether(wei)
    }

    pub fn from_eth(eth: f64) -> Self {
        let dec = BigDecimal::from_f64(eth)
            .unwrap_or_else(|| panic!("{} is an invalid eth value !", eth));
        Self::from_eth_bigdec(&dec)
    }

    pub fn from_wei(wei: U256) -> Self {
        Ether(wei)
    }

    pub fn ethereum(&self) -> f64 {
        self.0.to_float(18)
    }

    pub fn wei(&self) -> U256 {
        self.0
    }
    pub fn zero() -> Self {
        Self::from_wei(U256::zero())
    }
}

impl fmt::Display for Ether {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let nice_decimals = self.0.to_decimal_str(18);
        write!(f, "{} ETH", nice_decimals)
    }
}

impl FromStr for Ether {
    type Err = ParseBigDecimalError;
    fn from_str(string: &str) -> Result<Ether, Self::Err> {
        let dec = BigDecimal::from_str(string)?;
        Ok(Self::from_eth_bigdec(&dec))
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
                let wei = U256::from_decimal_str(v).map_err(E::custom)?;
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
        let (bigint, _exponent) = self.0.to_bigdec(18).as_bigint_and_exponent();
        serializer.serialize_str(bigint.to_string().as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::{asset, ethereum::U256};
    use lazy_static::lazy_static;
    use std::{f64, str::FromStr};

    lazy_static! {
        static ref WEI_IN_ETHEREUM: U256 = U256::from((10u64).pow(18));
    }

    #[test]
    fn display_ethereum() {
        assert_eq!(asset::Ether::from_eth(9000.0).to_string(), "9000 ETH");
    }

    #[test]
    fn a_ethereum_is_a_quintillion_wei() {
        assert_eq!(
            asset::Ether::from_eth(2.0).wei(),
            U256::from(2_000_000_000_000_000_000u64) // 2 quintillion
        )
    }

    #[test]
    fn from_eth_works_when_resulting_wei_cant_fit_in_u64() {
        assert_eq!(
            asset::Ether::from_eth(9001.0).wei(),
            U256::from(9001u64) * *WEI_IN_ETHEREUM
        )
    }

    #[test]
    fn from_fractional_ethereum_converts_to_correct_wei() {
        assert_eq!(
            asset::Ether::from_eth(0.000_000_001).wei(),
            U256::from(1_000_000_000)
        )
    }

    #[test]
    fn ether_quantity_from_str() {
        assert_eq!(
            asset::Ether::from_str("1.000000001").unwrap().wei(),
            U256::from(1_000_000_001_000_000_000u64)
        )
    }

    #[test]
    fn ether_quantity_back_into_f64() {
        assert!(asset::Ether::from_eth(0.1234).ethereum() - 0.1234f64 < f64::EPSILON)
    }

    #[test]
    fn fractional_ethereum_format() {
        assert_eq!(asset::Ether::from_eth(0.1234).to_string(), "0.1234 ETH")
    }

    #[test]
    fn whole_ethereum_format() {
        assert_eq!(asset::Ether::from_eth(12.0).to_string(), "12 ETH");
    }

    #[test]
    fn ethereum_with_small_fraction_format() {
        assert_eq!(
            asset::Ether::from_str("1234.00000100").unwrap().to_string(),
            "1234.000001 ETH"
        )
    }

    #[test]
    fn one_hundren_ethereum_format() {
        assert_eq!(
            asset::Ether::from_str("100").unwrap().to_string(),
            "100 ETH"
        )
    }

    #[test]
    fn serialize_ether_quantity() {
        let quantity = asset::Ether::from_eth(1.0);
        let quantity_str = serde_json::to_string(&quantity).unwrap();
        assert_eq!(quantity_str, "\"1000000000000000000\"");
    }

    #[test]
    fn deserialize_ether_quantity() {
        let quantity_str = "\"1000000000000000000\"";
        let quantity = serde_json::from_str::<asset::Ether>(quantity_str).unwrap();
        assert_eq!(quantity, asset::Ether::from_eth(1.0));
    }
}
