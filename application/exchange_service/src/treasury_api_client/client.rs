use bitcoin_support::*;
use common_types::TradingSymbol;
use ethereum_support::*;
use reqwest;

#[derive(Clone, Debug)]
pub struct TreasuryApiUrl(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct RateRequestBody {
    buy_amount: f64, //ethereum
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub sell_amount: BitcoinQuantity,
    pub buy_amount: EthereumQuantity,
}

pub trait ApiClient: Send + Sync {
    fn request_rate(
        &self,
        symbol: TradingSymbol,
        buy_amount: f64,
    ) -> Result<RateResponseBody, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: TreasuryApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn request_rate(
        &self,
        symbol: TradingSymbol,
        buy_amount: f64,
    ) -> Result<RateResponseBody, reqwest::Error> {
        self.client
            .get(format!("{}/rates/{}?amount={}", self.url.0, symbol, buy_amount).as_str())
            .send()
            .and_then(|mut res| res.json::<RateResponseBody>())
    }
}
