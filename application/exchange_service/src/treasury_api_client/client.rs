use common_types::TradingSymbol;
use reqwest;

#[derive(Clone, Debug)]
pub struct TreasuryApiUrl(pub String);

//TODO: Update the treasury service!
//TODO: Make it generic (if possible)
#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    pub rate: f64,
}

pub trait ApiClient: Send + Sync {
    fn request_rate(&self, symbol: TradingSymbol) -> Result<RateResponseBody, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: TreasuryApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn request_rate(&self, symbol: TradingSymbol) -> Result<RateResponseBody, reqwest::Error> {
        self.client
            .get(format!("{}/rates/{}", self.url.0, symbol).as_str())
            .send()
            .and_then(|mut res| res.json::<RateResponseBody>())
    }
}
