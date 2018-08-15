use bitcoin_rpc;
use common_types::TradingSymbol;
use ethereum_support;
use reqwest;
use secret::SecretHash;
use swaps::TradeId;

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

#[derive(Serialize, Deserialize)]
struct OfferRequestBody {
    amount: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: String,
    pub sell_amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub contract_secret_lock: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address, // todo change this to bitcoin_support
    pub client_success_address: ethereum_support::Address,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponseBody {
    pub exchange_refund_address: ethereum_support::Address,
    pub exchange_contract_time_lock: u64,
    pub exchange_success_address: bitcoin_rpc::Address, // todo change this to bitcoin_support
}

pub trait ApiClient: Send + Sync {
    fn create_offer(
        &self,
        symbol: TradingSymbol,
        amount: f64,
    ) -> Result<OfferResponseBody, reqwest::Error>;
    fn create_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody,
    ) -> Result<OrderResponseBody, reqwest::Error>;
}

pub struct DefaultApiClient {
    client: reqwest::Client,
    url: ExchangeApiUrl,
}

impl DefaultApiClient {
    pub fn new(url: ExchangeApiUrl) -> Self {
        DefaultApiClient {
            url,
            client: reqwest::Client::new(),
        }
    }
}

impl ApiClient for DefaultApiClient {
    fn create_offer(
        &self,
        symbol: TradingSymbol,
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
        symbol: TradingSymbol,
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
