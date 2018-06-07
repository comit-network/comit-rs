#[allow(unused_imports)]
use reqwest;

mod client;
mod fake_client;

pub use self::client::ApiClient;
pub use self::client::BuyOfferRequestBody;
pub use self::client::BuyOrderRequestBody;
pub use self::client::OfferResponseBody;
pub use self::client::RequestToFund;
pub use self::client::TradingApiUrl;

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
