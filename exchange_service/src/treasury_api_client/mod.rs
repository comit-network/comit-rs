mod client;
pub use self::client::*;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct Rate {
    pub symbol: Symbol,
    pub rate: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Symbol(pub String); // Expected format: ETH-BTC or LTC-BTC

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Export classes for test

#[cfg(test)]
mod fake_client;

#[cfg(test)]
pub use self::fake_client::*;
