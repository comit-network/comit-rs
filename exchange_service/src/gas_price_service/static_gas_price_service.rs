use gas_price_service::Error;
use gas_price_service::GasPriceService;
use web3::types::U256;

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
