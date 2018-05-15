use reqwest;
use types::*;

pub struct DefaultApiClient {
    pub url: ExchangeApiUrl,
    pub client: reqwest::Client,
}

pub trait ApiClient {
    fn request_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error>;
}

impl ApiClient for DefaultApiClient {
    fn request_offer(&self, request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .post(self.url.0.as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }
}
