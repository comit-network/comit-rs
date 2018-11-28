use bigdecimal::{BigDecimal, ParseBigDecimalError};
use num::FromPrimitive;
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{f64, fmt, str::FromStr};
use u256_ext::{FromBigUInt, FromDecimalStr, ToBigDecimal, ToDecimalStr, ToFloat};
use U256;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherQuantity(U256);

impl EtherQuantity {
    fn from_eth_bigdec(decimal: &BigDecimal) -> EtherQuantity {
        let (wei_bigint, _) = decimal.with_scale(18).as_bigint_and_exponent();
        let wei = U256::from_biguint(wei_bigint.to_biguint().unwrap());
        EtherQuantity(wei)
    }

    pub fn from_eth(eth: f64) -> Self {
        let dec = BigDecimal::from_f64(eth)
            .unwrap_or_else(|| panic!("{} is an invalid eth value !", eth));
        Self::from_eth_bigdec(&dec)
    }

    pub fn from_wei(wei: U256) -> Self {
        EtherQuantity(wei)
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

impl fmt::Display for EtherQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let nice_decimals = self.0.to_decimal_str(18);
        write!(f, "{} ETH", nice_decimals)
    }
}

impl FromStr for EtherQuantity {
    type Err = ParseBigDecimalError;
    fn from_str(string: &str) -> Result<EtherQuantity, Self::Err> {
        let dec = BigDecimal::from_str(string)?;
        Ok(Self::from_eth_bigdec(&dec))
    }
}

impl<'de> Deserialize<'de> for EtherQuantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = EtherQuantity;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("A string representing a wei quantity")
            }

            fn visit_str<E>(self, v: &str) -> Result<EtherQuantity, E>
            where
                E: de::Error,
            {
                let wei = U256::from_decimal_str(v).map_err(E::custom)?;
                Ok(EtherQuantity(wei))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for EtherQuantity {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let (bigint, _exponent) = self.0.to_bigdec(18).as_bigint_and_exponent();
        serializer.serialize_str(bigint.to_string().as_str())
    }
}
