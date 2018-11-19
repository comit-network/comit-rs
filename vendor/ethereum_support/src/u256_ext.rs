use bigdecimal::BigDecimal;
use num::{
    bigint::{BigInt, Sign},
    ToPrimitive,
};
use std::{f64, mem};
use web3::types::U256;

const U64SIZE: usize = mem::size_of::<u64>();

pub trait ToBigDecimal {
    fn to_bigdec(&self, decimals: i64) -> BigDecimal;
}

pub trait ToFloat {
    fn to_float(&self, decimals: i64) -> f64;
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

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn create_u256_with_18_dec_from_wei_to_full_token() {
        let number = U256::from(1_000_000_000_000_000_000u64);
        let float = 1.0;

        assert_that(&number.to_float(18)).is_equal_to(&float);
    }

    #[test]
    fn create_u256_with_16_dec_from_wei_to_full_token() {
        let number = U256::from(1_000_000_000_000_000_000u64);
        let float = 100.0;

        assert_that(&number.to_float(16)).is_equal_to(&float);
    }

}
