use bitcoin_fee_service::{BitcoinFeeService, Error};
use bitcoin_support::BitcoinQuantity;

pub struct StaticBitcoinFeeService(BitcoinQuantity);

impl BitcoinFeeService for StaticBitcoinFeeService {
    fn get_recommended_fee(&self) -> Result<BitcoinQuantity, Error> {
        Ok(self.0)
    }
}

impl StaticBitcoinFeeService {
    pub fn new(per_byte: BitcoinQuantity) -> Self {
        StaticBitcoinFeeService(per_byte)
    }
}
