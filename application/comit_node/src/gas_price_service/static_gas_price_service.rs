use ethereum_support::U256;
use gas_price_service::{Error, GasPriceService};

#[derive(Debug)]
pub struct StaticGasPriceService(U256);

impl Default for StaticGasPriceService {
    fn default() -> Self {
        StaticGasPriceService(U256::from(100))
    }
}

impl GasPriceService for StaticGasPriceService {
    fn get_gas_price(&self) -> Result<U256, Error> {
        Ok(self.0)
    }
}

impl StaticGasPriceService {
    pub fn new(gas: u64) -> Self {
        StaticGasPriceService(U256::from(gas))
    }
}
