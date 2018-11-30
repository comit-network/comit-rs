// TODO: contribute this to rust_bitcoin so it is returned by
// Transaction::get_weight
use bitcoin_quantity::BitcoinQuantity;

#[derive(Debug, PartialEq)]
pub struct Weight(u64);

impl Weight {
    pub fn to_virtual_bytes(&self) -> f64 {
        (self.0 as f64) / 4.0
    }

    pub fn calculate_fee(&self, sats_per_byte: f64) -> BitcoinQuantity {
        BitcoinQuantity::from_satoshi((self.to_virtual_bytes() * sats_per_byte).ceil() as u64)
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

    #[test]
    fn given_a_rate_and_transaction_size_can_calculate_estimated_fee() {
        let per_byte = 10.0;
        let fee = Weight::from(400_u64).calculate_fee(per_byte);
        assert_eq!(fee, BitcoinQuantity::from_satoshi(1000));
    }
}
