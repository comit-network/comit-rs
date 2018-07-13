use super::Symbol;
use super::client::ApiClient;
use bitcoin_support::BitcoinQuantity;
use common_types::EthereumQuantity;
use reqwest;
use treasury_api_client::RateResponseBody;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_rate(
        &self,
        symbol: Symbol,
        buy_amount: f64,
    ) -> Result<RateResponseBody, reqwest::Error> {
        let rate = 0.7;
        let sell_amount = BitcoinQuantity::from_bitcoin(buy_amount * rate);
        let buy_amount = EthereumQuantity::from_eth(buy_amount);
        Ok(RateResponseBody {
            symbol: symbol.to_string(),
            rate,
            sell_amount,
            buy_amount,
        })
    }
}
