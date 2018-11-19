use bigdecimal::{BigDecimal, ParseBigDecimalError};
use num::{bigint::BigUint, FromPrimitive};
use regex::Regex;
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{f64, fmt, str::FromStr};
use u256_ext::{ToBigDecimal, ToFloat};
use U256;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EtherQuantity(U256);

impl EtherQuantity {
    fn from_eth_bigdec(decimal: &BigDecimal) -> EtherQuantity {
        let (wei_bigint, _) = decimal.with_scale(18).as_bigint_and_exponent();
        Self::from_wei_bigint(&wei_bigint.to_biguint().unwrap())
    }

    pub fn from_eth(eth: f64) -> Self {
        let dec = BigDecimal::from_f64(eth)
            .unwrap_or_else(|| panic!("{} is an invalid eth value !", eth));
        Self::from_eth_bigdec(&dec)
    }

    pub fn from_wei(wei: U256) -> Self {
        EtherQuantity(wei)
    }

    fn from_wei_bigint(wei: &BigUint) -> EtherQuantity {
        let bytes = wei.to_bytes_be();
        let mut buf = [0u8; 32];
        let start = 32 - bytes.len();
        buf[start..].clone_from_slice(&bytes[..]);
        EtherQuantity(buf.into())
    }

    pub fn ethereum(&self) -> f64 {
        self.0.to_float(18)
    }

    pub fn wei(&self) -> U256 {
        self.0
    }
}

lazy_static! {
    static ref TRAILING_ZEROS: Regex = Regex::new(r"\.?0*$").unwrap();
}

impl fmt::Display for EtherQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // At time of writing BigDecimal always puts . and pads zeroes
        // up to the precision in f, so TRAILING_ZEROS does the right
        // thing in all cases.
        let fmt_dec = format!("{}", self.0.to_bigdec(18));
        let removed_trailing_zeros = TRAILING_ZEROS.replace(fmt_dec.as_str(), "");
        write!(f, "{} ETH", removed_trailing_zeros)
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
                let bigint = BigUint::from_str(v).map_err(E::custom)?;
                Ok(EtherQuantity::from_wei_bigint(&bigint))
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
