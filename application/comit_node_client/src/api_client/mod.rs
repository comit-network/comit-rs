#[allow(unused_imports)]
use reqwest;

mod client;
mod fake_client;

pub use self::client::{
    ApiClient, BuyOfferRequestBody, BuyOrderRequestBody, ComitNodeApiUrl, OfferResponseBody,
    RequestToFund, TradeId, TradingServiceError,
};

#[cfg(test)]
pub fn create_client(_url: &ComitNodeApiUrl) -> impl ApiClient {
    fake_client::FakeApiClient {}
}

#[cfg(not(test))]
pub fn create_client(url: &ComitNodeApiUrl) -> impl ApiClient {
    client::DefaultApiClient {
        client: reqwest::Client::new(),
        url: url.clone(),
    }
}
