use types::OfferRequest;
use types::Offer;
use types::ExchangeApiUrl;
use reqwest;

pub trait ApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error>;
}

pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: ExchangeApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn create_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        self.client
            .post(self.url.0.as_str())
            .json(offer_request)
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }
}