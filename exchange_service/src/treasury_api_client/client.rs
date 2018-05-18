use reqwest;
use types::{Rate, Symbol, TreasuryApiUrl};

pub trait ApiClient {
    fn request_rate(&self, symbol: Symbol) -> Result<Rate, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: TreasuryApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn request_rate(&self, symbol: Symbol) -> Result<Rate, reqwest::Error> {
        self.client
            .get(format!("{}/rate/{}", self.url.0, symbol.0).as_str())
            .send()
            .and_then(|mut res| res.json::<Rate>())
    }
}
