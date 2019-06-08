use bitcoin_quantity::BitcoinQuantity;

#[derive(Debug, PartialEq)]
pub enum Error {
    FeeTooHigh,
}

#[derive(Debug, PartialEq)]
pub struct Weight(u64);

#[allow(
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]
impl Weight {
    pub fn calculate_fee(&self, sats_per_wu: u64) -> Result<BitcoinQuantity, Error> {
        let sats = self
            .0
            .checked_mul(sats_per_wu)
            .ok_or_else(|| Error::FeeTooHigh)?;

        Ok(BitcoinQuantity::from_satoshi(sats))
    }
}

impl From<Weight> for u64 {
    fn from(weight: Weight) -> u64 {
        weight.0
    }
}

impl From<u64> for Weight {
    fn from(value: u64) -> Self {
        Weight(value)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn number_overflow() {
        let per_byte = 10;
        let fees = Weight::from(std::u64::MAX).calculate_fee(per_byte);
        assert_that(&fees).is_err_containing(Error::FeeTooHigh);
    }

    // Data taken from:
    // https://www.blockchain.com/btc/tx/8c7e14ed71e6821d941d102c2fe6ad56f4b12bbc9348e3de6872048f4cec17cf
    #[test]
    fn should_calculate_correct_fee() {
        let fee = Weight(574).calculate_fee(22).unwrap();

        let diff = (fee - BitcoinQuantity::from_satoshi(12578)).satoshi();

        // Unfortunately, we can't use the exact numbers from the above TX becaues of
        // float numbers.
        assert_that(&diff).is_less_than(100);
    }
}
