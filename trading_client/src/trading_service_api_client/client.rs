use reqwest;
use std::fmt;
use types::*;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[derive(Deserialize)]
pub struct RedeemDetails {
    pub uid: Uuid,
    pub address: EthAddress,
    pub gas: u32,
}

pub struct DefaultApiClient {
    pub url: TradingApiUrl,
    pub client: reqwest::Client,
}

pub trait ApiClient {
    fn request_offer(&self, offer_request: &OfferRequest) -> Result<Offer, reqwest::Error>;
    fn request_redeem_details(&self, uid: Uuid) -> Result<RedeemDetails, reqwest::Error>;
}

impl ApiClient for DefaultApiClient {
    fn request_offer(&self, request: &OfferRequest) -> Result<Offer, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/trades/ETH-BTC/buy-offers", self.url.0).as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<Offer>())
    }

    fn request_redeem_details(&self, uid: Uuid) -> Result<RedeemDetails, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .get(format!("{}/trades/ETH-BTC/{}/redeem-orders", self.url.0, uid).as_str())
            .send()
            .and_then(|mut res| res.json::<RedeemDetails>())
    }
}
