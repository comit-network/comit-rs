use super::client::ApiClient;
use reqwest;
use types::Offer;
use types::OfferRequest;
use types::Symbol;

pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        Ok(Offer {
            symbol: Symbol("ETH:BTC".to_string()),
            rate: 0.0,
            uid: String::new(),
        })
    }
}
