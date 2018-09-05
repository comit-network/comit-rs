mod client;
pub use self::client::*;
use common_types::TradingSymbol;

#[derive(Debug, Deserialize)]
pub struct Rate {
    pub symbol: TradingSymbol,
    pub rate: f64,
}

// Export classes for test

mod fake_client;

pub use self::fake_client::FakeApiClient;
