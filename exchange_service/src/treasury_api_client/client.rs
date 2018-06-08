use super::{Rate, Symbol};
use reqwest;

#[derive(Clone)]
pub struct TreasuryApiUrl(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct RateRequestBody {
    //TODO: make it work with float
    buy_amount: u32, //ethereum
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    pub symbol: String,
    pub rate: f32,
    pub sell_amount: u32, //satoshis
    pub buy_amount: u32,  //ethereum
}

pub trait ApiClient: Send + Sync {
    fn request_rate(
        &self,
        symbol: Symbol,
        buy_amount: u32,
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
        symbol: Symbol,
        buy_amount: u32,
    ) -> Result<RateResponseBody, reqwest::Error> {
        let body = RateRequestBody { buy_amount };

        self.client
            .post(format!("{}/rates/{}", self.url.0, symbol).as_str())
            .json(&body)
            .send()
            .and_then(|mut res| res.json::<RateResponseBody>())
    }
}
