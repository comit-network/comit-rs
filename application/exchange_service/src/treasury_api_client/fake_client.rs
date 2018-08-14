use super::client::ApiClient;
use bitcoin_support::BitcoinQuantity;
use common_types::TradingSymbol;
use ethereum_support::EthereumQuantity;
use reqwest;
use treasury_api_client::RateResponseBody;

#[allow(dead_code)]
pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_rate(
        &self,
        symbol: TradingSymbol,
        buy_amount: f64,
    ) -> Result<RateResponseBody, reqwest::Error> {
        let rate = 0.7;
        let sell_amount = BitcoinQuantity::from_bitcoin(buy_amount * rate);
        let buy_amount = EthereumQuantity::from_eth(buy_amount);
        Ok(RateResponseBody {
            symbol,
            rate,
            sell_amount,
            buy_amount,
        })
    }
}
