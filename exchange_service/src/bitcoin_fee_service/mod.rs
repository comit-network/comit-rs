mod static_bitcoin_fee_service;

pub use self::static_bitcoin_fee_service::StaticBitcoinFeeService;

#[derive(Debug)]
pub enum Error {
    Unavailable,
}

pub trait BitcoinFeeService: Send + Sync {
    fn get_recommended_fee(&self) -> Result<f64, Error>;
}
