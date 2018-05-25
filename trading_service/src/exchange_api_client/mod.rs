mod client;
mod fake_client;

pub use self::client::{ApiClient, ExchangeApiUrl, Offer, TradeRequestBody};

#[cfg(test)]
pub fn create_client(_url: &ExchangeApiUrl) -> impl ApiClient {
    fake_client::FakeApiClient {}
}

#[cfg(not(test))]
pub fn create_client(url: &ExchangeApiUrl) -> impl ApiClient {
    use reqwest;
    client::DefaultApiClient {
        client: reqwest::Client::new(),
        url: url.clone(),
    }
}
