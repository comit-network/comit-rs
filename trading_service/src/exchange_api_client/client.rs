use offer::Offer;
use reqwest;
use symbol::Symbol;

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

pub trait ApiClient {
    fn create_offer(&self, symbol: Symbol, amount: u32) -> Result<Offer, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: ExchangeApiUrl,
}

#[derive(Serialize, Deserialize)]
struct OfferRequestBody {
    amount: u32,
}

impl ApiClient for DefaultApiClient {
    fn create_offer(&self, symbol: Symbol, amount: u32) -> Result<Offer, reqwest::Error> {
        let body = OfferRequestBody { amount };

        self.client
            .post(format!("{}/trades/{}/buy-offers", self.url.0, symbol).as_str())
            .json(&body)
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }
}
