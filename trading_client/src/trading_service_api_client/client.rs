use bitcoin_rpc;
use offer::Symbol;
use reqwest;
use std::fmt;
use std::str::FromStr;
use uuid::ParseError;
use uuid::Uuid;
use web3::types::Address as EthAddress;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl FromStr for TradeId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let uid = Uuid::from_str(s)?;
        Ok(TradeId(uid))
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
pub struct TradingApiUrl(pub String);

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub url: TradingApiUrl,
    pub client: reqwest::Client,
}

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
    pub uid: TradeId,
    pub symbol: Symbol,
    pub amount: u32,
    pub rate: f32,
    pub sell_amount: u32,
}

#[derive(Deserialize, Serialize)]
pub struct BuyOrderRequestBody {
    client_success_address: EthAddress,
    client_refund_address: bitcoin_rpc::Address,
}

impl BuyOrderRequestBody {
    pub fn new(
        client_success_address: String,
        client_refund_address: String,
    ) -> BuyOrderRequestBody {
        let mut client_success_address = client_success_address.clone();
        if client_success_address.starts_with("0x") {
            // Need to strip it out
            client_success_address.remove(0);
            client_success_address.remove(0);
        };

        let client_success_address = EthAddress::from_str(client_success_address.as_str())
            .expect("Could not convert the success address");
        let client_refund_address = bitcoin_rpc::Address::from(client_refund_address.as_str());

        BuyOrderRequestBody {
            client_success_address,
            client_refund_address,
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct RequestToFund {
    pub address_to_fund: bitcoin_rpc::Address,
    pub sell_amount: u32,
}

#[derive(Deserialize)]
pub struct RedeemDetails {
    pub uid: Uuid,
    pub address: EthAddress,
    pub gas: u32,
}

pub trait ApiClient {
    fn request_offer(
        &self,
        symbol: &Symbol,
        offer_request: &BuyOfferRequestBody,
    ) -> Result<OfferResponseBody, reqwest::Error>;

    fn request_order(
        &self,
        symbol: &Symbol,
        uid: Uuid,
        request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, reqwest::Error>;

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

    fn request_order(
        &self,
        symbol: &Symbol,
        uid: Uuid,
        request: &BuyOrderRequestBody,
    ) -> Result<RequestToFund, reqwest::Error> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/trades/{}/{}/buy-orders", self.url.0, symbol, uid).as_str())
            .json(request)
            .send()
            .and_then(|mut res| res.json::<RequestToFund>())
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
