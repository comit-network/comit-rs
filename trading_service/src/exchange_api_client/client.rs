use bitcoin_rpc;
use btc_htlc::BtcBlockHeight;
use reqwest;
use secret::SecretHash;
use stub::{EthAddress, EthTimeDelta};
use symbol::Symbol;
use uuid::Uuid;

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

#[derive(Serialize, Deserialize)]
struct OfferRequestBody {
    amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub uid: Uuid,
    pub secret_hash: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: EthAddress,
    pub long_relative_timelock: BtcBlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponseBody {
    //Indicates the order was "taken"
    pub uid: Uuid,
    pub exchange_refund_address: EthAddress,
    pub short_relative_timelock: EthTimeDelta,
    pub exchange_success_address: bitcoin_rpc::Address,
}

pub trait ApiClient {
    fn create_offer(
        &self,
        symbol: Symbol,
        amount: u32,
    ) -> Result<OfferResponseBody, reqwest::Error>;
    fn create_trade(
        &self,
        symbol: Symbol,
        &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error>;
}

#[allow(dead_code)]
pub struct DefaultApiClient {
    pub client: reqwest::Client,
    pub url: ExchangeApiUrl,
}

impl ApiClient for DefaultApiClient {
    fn create_offer(
        &self,
        symbol: Symbol,
        amount: u32,
    ) -> Result<OfferResponseBody, reqwest::Error> {
        let body = OfferRequestBody { amount };

        self.client
            .post(format!("{}/trades/{}/buy-offers", self.url.0, symbol).as_str())
            .json(&body)
            .send()
            .and_then(|mut res| res.json::<OfferResponseBody>())
    }

    fn create_trade(
        &self,
        symbol: Symbol,
        trade_request: &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error> {
        self.client
            .post(
                format!(
                    "{}/trades/{}/{}/buy-orders",
                    self.url.0, symbol, trade_request.uid
                ).as_str(),
            )
            .json(trade_request)
            .send()
            .and_then(|mut res| res.json::<OrderResponseBody>())
    }
}
