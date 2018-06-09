use super::client::ApiClient;
use super::{Rate, Symbol};
use reqwest;
use treasury_api_client::RateResponseBody;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_rate(
        &self,
        symbol: Symbol,
        buy_amount: u64,
    ) -> Result<RateResponseBody, reqwest::Error> {
        let rate = 0.7;
        let sell_amount = (buy_amount as f64 * rate).round().abs() as u64;
        Ok(RateResponseBody {
            symbol: symbol.to_string(),
            rate,
            sell_amount,
            buy_amount,
        })
    }
}
