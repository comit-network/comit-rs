use bitcoin_fee_service::{BitcoinFeeService, Error, Rate};
use common_types::BitcoinQuantity;

pub struct StaticBitcoinFeeService(Rate);

impl BitcoinFeeService for StaticBitcoinFeeService {
    fn get_recommended_fee(&self) -> Result<Rate, Error> {
        Ok(self.0)
    }
}

impl StaticBitcoinFeeService {
    pub fn new(per_byte: BitcoinQuantity) -> Self {
        StaticBitcoinFeeService(Rate(per_byte))
    }
}
