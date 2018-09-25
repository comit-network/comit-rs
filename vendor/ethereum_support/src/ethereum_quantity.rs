use bigdecimal::{BigDecimal, ParseBigDecimalError};
use byteorder::{LittleEndian, WriteBytesExt};
use num::{
    bigint::{BigInt, BigUint, Sign},
    FromPrimitive, ToPrimitive,
};
use regex::Regex;
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{f64, fmt, mem, str::FromStr};
use U256;

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct EthereumQuantity(U256);

const U64SIZE: usize = mem::size_of::<u64>();

impl EthereumQuantity {
    fn from_eth_bigdec(decimal: &BigDecimal) -> EthereumQuantity {
        let (wei_bigint, _) = decimal.with_scale(18).as_bigint_and_exponent();
        Self::from_wei_bigint(&wei_bigint.to_biguint().unwrap())
    }

    pub fn from_eth(eth: f64) -> Self {
        let dec =
            //.unwrap_or_else(|| panic!(format!("{} is an invalid eth value", eth).as_str()));;
            BigDecimal::from_f64(eth).expect(format!("{} is an invalid eth value", eth).as_str());
        Self::from_eth_bigdec(&dec)
    }

    pub fn from_wei(wei: U256) -> Self {
        EthereumQuantity(wei)
    }

    fn from_wei_bigint(wei: &BigUint) -> EthereumQuantity {
        let bytes = wei.to_bytes_be();
        let mut buf = [0u8; 32];
        let start = 32 - bytes.len();
        buf[start..].clone_from_slice(&bytes[..]);
        EthereumQuantity(buf.into())
    }

    fn to_ethereum_bigdec(&self) -> BigDecimal {
        let mut bs = [0u8; U64SIZE * 4];

        let _u256 = self.0;
        let four_u64s_little_endian = _u256.0;

        for index in 0..4 {
            let _u64 = four_u64s_little_endian[index];
            let start = index * U64SIZE;
            let end = (index + 1) * U64SIZE;
            bs[start..end]
                .as_mut()
                .write_u64::<LittleEndian>(_u64)
                .expect("Unable to write");
        }

        let bigint = BigInt::from_bytes_le(Sign::Plus, &bs);

        BigDecimal::new(bigint, 18)
    }

    pub fn ethereum(&self) -> f64 {
        self.to_ethereum_bigdec().to_f64().unwrap()
    }

    pub fn wei(&self) -> U256 {
        self.0
    }
}

lazy_static! {
    static ref TRAILING_ZEROS: Regex = Regex::new(r"\.?0*$").unwrap();
}

impl fmt::Display for EthereumQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // At time of writing BigDecimal always puts . and pads zeroes
        // up to the precision in f, so TRAILING_ZEROS does the right
        // thing in all cases.
        let fmt_dec = format!("{}", self.to_ethereum_bigdec());
        let removed_trailing_zeros = TRAILING_ZEROS.replace(fmt_dec.as_str(), "");
        write!(f, "{} ETH", removed_trailing_zeros)
    }
}

impl FromStr for EthereumQuantity {
    type Err = ParseBigDecimalError;
    fn from_str(string: &str) -> Result<EthereumQuantity, Self::Err> {
        let dec = BigDecimal::from_str(string)?;
        Ok(Self::from_eth_bigdec(&dec))
    }
}

impl<'de> Deserialize<'de> for EthereumQuantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = EthereumQuantity;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("A string representing a wei quantity")
            }

            fn visit_str<E>(self, v: &str) -> Result<EthereumQuantity, E>
            where
                E: de::Error,
            {
                let bigint = BigUint::from_str(v).map_err(E::custom)?;
                Ok(EthereumQuantity::from_wei_bigint(&bigint))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for EthereumQuantity {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let (bigint, _exponent) = self.to_ethereum_bigdec().as_bigint_and_exponent();
        serializer.serialize_str(bigint.to_string().as_str())
    }
}
