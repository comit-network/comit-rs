use super::client::ApiClient;
use common_types::TradingSymbol;
use reqwest;
use treasury_api_client::RateResponseBody;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_rate(&self, _symbol: TradingSymbol) -> Result<RateResponseBody, reqwest::Error> {
        let rate = 0.7;
        Ok(RateResponseBody { rate })
    }
}
