use super::client::ApiClient;
use reqwest;
use types::Offer;
use types::OfferRequest;

pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        unimplemented!()
    }
}
