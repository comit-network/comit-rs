use offer::Symbol;
use reqwest;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[derive(Clone)]
pub struct TradingApiUrl(pub String);

#[derive(Deserialize, Serialize)]
pub struct BuyOfferRequestBody {
    amount: u32,
}

impl BuyOfferRequestBody {
    pub fn new(amount: u32) -> BuyOfferRequestBody {
        BuyOfferRequestBody { amount }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferResponseBody {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
}

#[derive(Deserialize)]
pub struct RedeemDetails {
    pub uid: Uuid,
    pub address: EthAddress,
    pub gas: u32,
}

#[derive(Serialize, Deserialize)]
pub struct OfferResponseBodyOffer {
    symbol: String,
    rate: f32,
    uid: String,
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub url: TradingApiUrl,
    pub client: reqwest::Client,
}

pub trait ApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        offer_request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, reqwest::Error>;
    fn request_redeem_details(
        &self,
        symbol: Symbol,
        uid: Uuid,
    ) -> Result<RedeemDetails, reqwest::Error>;
}

impl ApiClient for DefaultApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/trades/{}/buy-offers", self.url.0, symbol).as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<OfferResponseBody>())
    }

    fn request_redeem_details(
        &self,
        symbol: Symbol,
        uid: Uuid,
    ) -> Result<RedeemDetails, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .get(format!("{}/trades/{}/{}/redeem-orders", self.url.0, symbol, uid).as_str())
            .send()
            .and_then(|mut res| res.json::<RedeemDetails>())
    }
}
