use super::client::ApiClient;
use types::OfferRequest;
use types::Offer;
use reqwest;

pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        unimplemented!()
    }
}