use super::client::ApiClient;
//use bitcoin_rpc::Address;
use reqwest;
use types::{Rate, Symbol};
//use uuid::Uuid;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_rate(&self, symbol: Symbol) -> Result<Rate, reqwest::Error> {
        let rate = Rate {
            symbol,
            rate: 3.14159,
        };
        Ok(rate)
    }
}
