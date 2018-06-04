use reqwest;
use types::TradingApiUrl;

mod client;
mod fake_client;

pub use self::client::ApiClient;

#[cfg(test)]
pub fn create_client(url: &TradingApiUrl) -> impl ApiClient {
    fake_client::FakeApiClient {}
}

#[cfg(not(test))]
pub fn create_client(url: &TradingApiUrl) -> impl ApiClient {
    client::DefaultApiClient {
        client: reqwest::Client::new(),
        url: url.clone(),
    }
}
