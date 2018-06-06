#[allow(unused_imports)]
use reqwest;
use types::TradingApiUrl;

mod client;
mod fake_client;

pub use self::client::ApiClient;
pub use self::client::BuyOfferRequestBody;
pub use self::client::OfferResponseBody;

#[cfg(test)]
pub fn create_client(_url: &TradingApiUrl) -> impl ApiClient {
    fake_client::FakeApiClient {}
}

#[cfg(not(test))]
pub fn create_client(url: &TradingApiUrl) -> impl ApiClient {
    client::DefaultApiClient {
        client: reqwest::Client::new(),
        url: url.clone(),
    }
}
