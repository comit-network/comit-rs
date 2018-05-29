mod client;
mod fake_client;

pub use self::client::ApiClient;
use rocket_factory::TreasuryApiUrl;

#[derive(Debug, Deserialize)]
pub struct Rate {
    pub symbol: Symbol,
    pub rate: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Symbol(pub String); // Expected format: ETH-BTC or LTC-BTC

#[cfg(test)]
pub fn create_client(_url: &TreasuryApiUrl) -> impl ApiClient {
    fake_client::FakeApiClient {}
}

#[cfg(not(test))]
pub fn create_client(url: &TreasuryApiUrl) -> impl ApiClient {
    use reqwest;
    client::DefaultApiClient {
        client: reqwest::Client::new(),
        url: url.clone(),
    }
}
