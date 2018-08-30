use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    TradingSymbol,
};
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
pub struct OfferResponseBody<B: Ledger, S: Ledger> {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: B::Quantity,
    pub sell_amount: S::Quantity,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody<B: Ledger, S: Ledger> {
    pub contract_secret_lock: SecretHash,
    pub client_refund_address: S::Address,
    pub client_success_address: B::Address,
    pub client_contract_time_lock: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponseBody<B: Ledger, S: Ledger> {
    pub exchange_refund_address: B::Address,
    pub exchange_contract_time_lock: u32,
    pub exchange_success_address: S::Address,
}

pub trait ApiClient: Send + Sync {
    fn create_buy_offer(
        &self,
        symbol: TradingSymbol,
        amount: f64,
    ) -> Result<OfferResponseBody<Ethereum, Bitcoin>, reqwest::Error>;
    fn create_buy_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, reqwest::Error>;
    fn create_sell_offer(
        &self,
        symbol: TradingSymbol,
        amount: f64,
    ) -> Result<OfferResponseBody<Bitcoin, Ethereum>, reqwest::Error>;
    fn create_sell_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Bitcoin, Ethereum>,
    ) -> Result<OrderResponseBody<Bitcoin, Ethereum>, reqwest::Error>;
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
    fn create_buy_offer(
        &self,
        symbol: TradingSymbol,
        amount: f64,
    ) -> Result<OfferResponseBody<Ethereum, Bitcoin>, reqwest::Error> {
        let body = OfferRequestBody { amount };

        self.client
            .post(format!("{}/trades/{}/buy-offers", self.url.0, symbol).as_str())
            .json(&body)
            .send()
            .and_then(|mut res| res.json::<OfferResponseBody<Ethereum, Bitcoin>>())
    }

    fn create_buy_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, reqwest::Error> {
        self.client
            .post(format!("{}/trades/{}/{}/buy-orders", self.url.0, symbol, uid).as_str())
            .json(trade_request)
            .send()
            .and_then(|mut res| res.json::<OrderResponseBody<Ethereum, Bitcoin>>())
    }

    fn create_sell_offer(
        &self,
        symbol: TradingSymbol,
        amount: f64,
    ) -> Result<OfferResponseBody<Bitcoin, Ethereum>, reqwest::Error> {
        let body = OfferRequestBody { amount };

        self.client
            .post(format!("{}/trades/{}/sell-offers", self.url.0, symbol).as_str())
            .json(&body)
            .send()
            .and_then(|mut res| res.json::<OfferResponseBody<Bitcoin, Ethereum>>())
    }

    fn create_sell_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Bitcoin, Ethereum>,
    ) -> Result<OrderResponseBody<Bitcoin, Ethereum>, reqwest::Error> {
        self.client
            .post(format!("{}/trades/{}/{}/sell-orders", self.url.0, symbol, uid).as_str())
            .json(trade_request)
            .send()
            .and_then(|mut res| res.json::<OrderResponseBody<Bitcoin, Ethereum>>())
    }
}
