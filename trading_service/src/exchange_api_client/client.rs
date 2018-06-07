use bitcoin_rpc;
use event_store::TradeId;
use reqwest;
use secret::SecretHash;
use symbol::Symbol;
use web3::types::Address;

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

#[derive(Serialize, Deserialize)]
struct OfferRequestBody {
    amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: TradeId,
    pub symbol: Symbol,
    pub amount: u32,
    pub rate: f32,
    pub sell_amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub contract_secret_lock: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: Address,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponseBody {
    pub exchange_refund_address: Address,
    pub exchange_contract_time_lock: u64,
    pub exchange_success_address: bitcoin_rpc::Address,
}

pub trait ApiClient {
    fn create_offer(
        &self,
        symbol: Symbol,
        amount: u32,
    ) -> Result<OfferResponseBody, reqwest::Error>;
    fn create_order(
        &self,
        symbol: Symbol,
        uid: TradeId,
        trade_request: &OrderRequestBody,
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

    fn create_order(
        &self,
        symbol: Symbol,
        uid: TradeId,
        trade_request: &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error> {
        self.client
            .post(format!("{}/trades/{}/{}/buy-orders", self.url.0, symbol, uid).as_str())
            .json(trade_request)
            .send()
            .and_then(|mut res| res.json::<OrderResponseBody>())
    }
}
