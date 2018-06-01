use super::client::ApiClient;
use reqwest;
use types::Offer;
use types::OfferRequest;

pub struct FakeApiClient;

impl ApiClient for FakeApiClient {
    fn request_offer(&self, _offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        unimplemented!()
    }
}
