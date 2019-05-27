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
    pub fn to_virtual_bytes(&self) -> f64 {
        (self.0 as f64) / 4.0
    }

    pub fn calculate_fee(&self, sats_per_byte: f64) -> Result<BitcoinQuantity, Error> {
        let sats = (self.to_virtual_bytes() * sats_per_byte).ceil();
        if sats > std::u64::MAX as f64 {
            return Err(Error::FeeTooHigh);
        }
        Ok(BitcoinQuantity::from_satoshi(sats as u64))
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
    fn given_a_rate_and_transaction_size_can_calculate_estimated_fee() {
        let per_byte = 10.0;
        let fees = Weight::from(400_u64).calculate_fee(per_byte);
        assert_that(&fees).is_ok_containing(BitcoinQuantity::from_satoshi(1000));
    }

    #[test]
    fn number_overflow_due_to_casting() {
        let per_byte = 10.0;
        let fees = Weight::from(std::u64::MAX).calculate_fee(per_byte);
        assert_that(&fees).is_err_containing(Error::FeeTooHigh);
    }
}
