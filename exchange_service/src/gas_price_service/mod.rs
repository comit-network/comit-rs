use ethereum_support::U256;

#[derive(Debug)]
pub enum Error {
    Unavailable,
}

pub trait GasPriceService: Send + Sync {
    fn get_gas_price(&self) -> Result<U256, Error>;
}

mod static_gas_price_service;

pub use self::static_gas_price_service::*;
