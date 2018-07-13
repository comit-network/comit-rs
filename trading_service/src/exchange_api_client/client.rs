use bitcoin_rpc;
use common_types::{BitcoinQuantity, EthereumQuantity};
use ethereum_support;
use event_store::TradeId;
use reqwest;
use secret::SecretHash;
use symbol::Symbol;

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

#[derive(Serialize, Deserialize)]
struct OfferRequestBody {
    amount: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: TradeId,
    pub symbol: Symbol,
    pub rate: f64,
    pub eth_amount: EthereumQuantity,
    pub btc_amount: BitcoinQuantity,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub contract_secret_lock: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: ethereum_support::Address,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponseBody {
    pub exchange_refund_address: ethereum_support::Address,
    pub exchange_contract_time_lock: u64,
    pub exchange_success_address: bitcoin_rpc::Address,
}

pub trait ApiClient {
    fn create_offer(
        &self,
        symbol: Symbol,
        amount: f64,
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
        amount: f64,
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
