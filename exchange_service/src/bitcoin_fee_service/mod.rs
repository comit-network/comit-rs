use bitcoin_wallet::Weight;
use common_types::BitcoinQuantity;

mod static_bitcoin_fee_service;

pub use self::static_bitcoin_fee_service::StaticBitcoinFeeService;

#[derive(Debug)]
pub enum Error {
    Unavailable,
}

#[derive(Copy, Clone)]
pub struct Rate(BitcoinQuantity);

impl Rate {
    pub fn calculate_fee_for_tx_with_weight(&self, tx_weight: Weight) -> BitcoinQuantity {
        let fee_per_byte = self.0.satoshi();
        let virtual_bytes = tx_weight.to_virtual_bytes();

        let tx_cost = fee_per_byte * virtual_bytes;

        BitcoinQuantity::from_satoshi(tx_cost)
    }
}

pub trait BitcoinFeeService: Send + Sync {
    /// Returns the currently recommended fee in bitcoin per satoshi
    fn get_recommended_fee(&self) -> Result<Rate, Error>;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn given_a_rate_and_transaction_size_can_calculate_estimated_fee() {
        let per_byte = BitcoinQuantity::from_satoshi(10);
        let rate = Rate(per_byte);

        let fee = rate.calculate_fee_for_tx_with_weight(Weight::from(400_u64));

        assert_eq!(fee, BitcoinQuantity::from_satoshi(1000));
    }
}
