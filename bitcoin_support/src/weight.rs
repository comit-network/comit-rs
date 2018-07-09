// TODO: contribute this to rust_bitcoin so it is returned by Transaction::get_weight
use bitcoin_quantity::BitcoinQuantity;

#[derive(Debug, PartialEq)]
pub struct Weight(u64);

impl Weight {
    pub fn to_virtual_bytes(&self) -> u64 {
        ((self.0 as f64) / 4.0).ceil() as u64
    }

    pub fn calculate_fee(&self, fee_per_byte: BitcoinQuantity) -> BitcoinQuantity {
        BitcoinQuantity::from_satoshi(self.to_virtual_bytes() * fee_per_byte.satoshi())
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
        let per_byte = BitcoinQuantity::from_satoshi(10);
        let fee = Weight::from(400_u64).calculate_fee(per_byte);
        assert_eq!(fee, BitcoinQuantity::from_satoshi(1000));
    }
}
