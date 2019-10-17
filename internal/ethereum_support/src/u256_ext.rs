use crate::web3::types::U256;
use bigdecimal::BigDecimal;
use lazy_static::lazy_static;
use num::{
    bigint::{BigInt, ParseBigIntError, Sign},
    BigUint, ToPrimitive,
};
use regex::Regex;
use std::{f64, mem};

const U64SIZE: usize = mem::size_of::<u64>();

lazy_static! {
    static ref TRAILING_ZEROS: Regex = Regex::new(r"\.?0*$").unwrap();
}

pub trait ToBigDecimal {
    fn to_bigdec(&self, decimals: i64) -> BigDecimal;
}

pub trait ToFloat {
    fn to_float(&self, decimals: i64) -> f64;
}

pub trait ToDecimalStr {
    fn to_decimal_str(&self, decimals: i64) -> String;
}

pub trait FromDecimalStr
where
    Self: Sized,
{
    type Err;

    fn from_decimal_str(value: &str) -> Result<Self, Self::Err>;
}

pub trait FromBigUInt
where
    Self: Sized,
{
    fn from_biguint(big_uint: BigUint) -> Self;
}

pub trait ToBigInt
where
    Self: Sized,
{
    fn to_bigint(&self) -> BigInt;
}

impl ToBigDecimal for U256 {
    fn to_bigdec(&self, scale: i64) -> BigDecimal {
        let big_int = self.to_bigint();
        BigDecimal::new(big_int, scale)
    }
}

impl ToFloat for U256 {
    fn to_float(&self, decimals: i64) -> f64 {
        self.to_bigdec(decimals).to_f64().unwrap()
    }
}

impl ToDecimalStr for U256 {
    fn to_decimal_str(&self, scale: i64) -> String {
        // At time of writing BigDecimal always puts . and pads zeroes
        // up to the precision in f, so TRAILING_ZEROS does the right
        // thing in all cases.
        let fmt_dec = self.to_bigdec(scale).to_string();
        TRAILING_ZEROS.replace(fmt_dec.as_str(), "").to_string()
    }
}

impl FromDecimalStr for U256 {
    type Err = ParseBigIntError;

    fn from_decimal_str(value: &str) -> Result<Self, Self::Err> {
        let big_unit = value.parse()?;
        Ok(U256::from_biguint(big_unit))
    }
}

impl FromBigUInt for U256 {
    fn from_biguint(big_unit: BigUint) -> Self {
        let bytes = big_unit.to_bytes_be();
        let mut buf = [0u8; 32];
        let start = 32 - bytes.len();
        buf[start..].clone_from_slice(&bytes[..]);
        U256::from(buf)
    }
}

impl ToBigInt for U256 {
    fn to_bigint(&self) -> BigInt {
        let mut bytes = [0u8; U64SIZE * 4];
        self.to_little_endian(&mut bytes);
        BigInt::from_bytes_le(Sign::Plus, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn given_u256_with_18_0_to_float_18_will_return_1() {
        let number = U256::from(1_000_000_000_000_000_000u64);
        let float = 1.0;

        assert_that(&number.to_float(18)).is_equal_to(&float);
    }

    #[test]
    fn given_u256_with_18_0_to_float_16_will_return_100() {
        let number = U256::from(1_230_000_000_000_000_000u64);
        let float = 123.0;

        assert_that(&number.to_float(16)).is_equal_to(&float);
    }

    #[test]
    fn given_u256_with_18_0_will_remove_18_trailling_0() {
        let number = U256::from(1_000_000_000_000_000_000u64);
        let string = String::from("100");

        assert_that(&number.to_decimal_str(16)).is_equal_to(&string);
    }
}
