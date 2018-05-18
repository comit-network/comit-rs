use types::TreasuryApiUrl;

mod client;
mod fake_client;

pub use self::client::ApiClient;

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
