use bigdecimal::BigDecimal;
use num::{
    bigint::{BigInt, Sign},
    ToPrimitive,
};
use regex::Regex;
use std::{f64, mem};
use web3::types::U256;

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

pub trait RemoveTrailingZeros {
    fn remove_trailing_zeros(&self, decimals: i64) -> String;
}

impl ToBigDecimal for U256 {
    fn to_bigdec(&self, decimals: i64) -> BigDecimal {
        let mut bytes = [0u8; U64SIZE * 4];
        self.to_little_endian(&mut bytes);
        let bigint = BigInt::from_bytes_le(Sign::Plus, &bytes);
        BigDecimal::new(bigint, decimals)
    }
}

impl ToFloat for U256 {
    fn to_float(&self, decimals: i64) -> f64 {
        self.to_bigdec(decimals).to_f64().unwrap()
    }
}

impl RemoveTrailingZeros for U256 {
    fn remove_trailing_zeros(&self, decimals: i64) -> String {
        // At time of writing BigDecimal always puts . and pads zeroes
        // up to the precision in f, so TRAILING_ZEROS does the right
        // thing in all cases.
        let fmt_dec = format!("{}", self.to_bigdec(decimals));
        TRAILING_ZEROS.replace(fmt_dec.as_str(), "").to_string()
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

        assert_that(&number.remove_trailing_zeros(16)).is_equal_to(&string);
    }

}
