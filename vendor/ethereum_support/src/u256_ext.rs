use bigdecimal::BigDecimal;
use num::bigint::{BigInt, Sign};
use std::mem;
use web3::types::U256;

const U64SIZE: usize = mem::size_of::<u64>();

pub trait ToBigDecimal {
    fn to_bigdec(&self, decimals: i64) -> BigDecimal;
}

impl ToBigDecimal for U256 {
    fn to_bigdec(&self, decimals: i64) -> BigDecimal {
        let mut bytes = [0u8; U64SIZE * 4];
        self.to_little_endian(&mut bytes);

        let bigint = BigInt::from_bytes_le(Sign::Plus, &bytes);

        BigDecimal::new(bigint, decimals)
    }
}
