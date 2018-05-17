use reqwest;
use types::ExchangeApiUrl;
use types::Offer;
use types::OfferRequest;

pub trait ApiClient {
    fn request_rate(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: ExchangeApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn request_rate(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        self.client
            .post(format!("{}/offers", self.url.0).as_str())
            .json(offer_request)
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }
}
