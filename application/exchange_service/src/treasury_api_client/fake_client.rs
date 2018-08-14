use super::client::ApiClient;
use common_types::TradingSymbol;
use reqwest;
use treasury_api_client::RateResponseBody;

#[allow(dead_code)]
pub struct FakeBuyApiClient;

impl ApiClient for FakeBuyApiClient {
    fn request_rate(
        &self,
        symbol: TradingSymbol,
        buy_amount: f64,
    ) -> Result<RateResponseBody, reqwest::Error> {
        let rate = 0.7;
        let sell_amount = buy_amount * rate;
        let buy_amount = buy_amount;
        Ok(RateResponseBody {
            symbol,
            rate,
            sell_amount,
            buy_amount,
        })
    }
}

#[allow(dead_code)]
pub struct FakeSellApiClient;

impl ApiClient for FakeSellApiClient {
    fn request_rate(
        &self,
        symbol: TradingSymbol,
        buy_amount: f64,
    ) -> Result<RateResponseBody, reqwest::Error> {
        let rate = 14.285;
        let sell_amount = buy_amount * rate;
        let buy_amount = buy_amount;
        Ok(RateResponseBody {
            symbol,
            rate,
            sell_amount,
            buy_amount,
        })
    }
}
