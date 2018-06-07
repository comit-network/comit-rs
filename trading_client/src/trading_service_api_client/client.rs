use bitcoin_rpc;
use offer::Symbol;
use reqwest;
use std::str::FromStr;
use uuid::Uuid;
use web3::types::Address as EthAddress;

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
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
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
    //TODO: specify amount of BTC
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
