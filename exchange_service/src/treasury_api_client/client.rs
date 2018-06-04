use super::{Rate, Symbol};
use reqwest;

#[derive(Clone)]
pub struct TreasuryApiUrl(pub String);

pub trait ApiClient: Send + Sync {
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
