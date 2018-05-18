use reqwest;
use types::{ExchangeApiUrl, Offer, OfferRequest, OfferRequestBody};

pub trait ApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: ExchangeApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        self.client
            .post(format!("{}/trades/{}/buy-offers", self.url.0, offer_request.symbol).as_str())
            .json(&OfferRequestBody::new(offer_request))
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }
}
