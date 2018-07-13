use bitcoin_fee_service::{BitcoinFeeService, Error};

pub struct StaticBitcoinFeeService(f64);

impl BitcoinFeeService for StaticBitcoinFeeService {
    fn get_recommended_fee(&self) -> Result<f64, Error> {
        Ok(self.0)
    }
}

impl StaticBitcoinFeeService {
    pub fn new(satoshi_per_byte: f64) -> Self {
        StaticBitcoinFeeService(satoshi_per_byte)
    }
}
