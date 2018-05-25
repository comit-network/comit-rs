use bitcoin_rpc;
use event_store::TradeCreated;
use reqwest;
use secret::SecretHash;
use stub::{BtcBlockHeight, EthAddress, EthTimeDelta};
use symbol::Symbol;
use uuid::Uuid;

#[derive(Clone)]
pub struct ExchangeApiUrl(pub String);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Offer {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32,
    // TODO: There is no reason for this to be here.
    // It can be moved to TradeAcceptance
    pub exchange_success_address: bitcoin_rpc::Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeAcceptance {
    pub uid: Uuid,
    pub exchange_refund_address: EthAddress,
    pub short_relative_timelock: EthTimeDelta,
}

pub trait ApiClient {
    fn create_offer(&self, symbol: Symbol, amount: u32) -> Result<Offer, reqwest::Error>;
    fn create_trade(
        &self,
        symbol: Symbol,
        &TradeRequestBody,
    ) -> Result<TradeAcceptance, reqwest::Error>;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeRequestBody {
    pub uid: Uuid,
    pub secret_hash: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: EthAddress,
    pub long_relative_timelock: BtcBlockHeight,
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

    fn create_trade(
        &self,
        symbol: Symbol,
        trade_request: &TradeRequestBody,
    ) -> Result<TradeAcceptance, reqwest::Error> {
        self.client
            .post(
                format!(
                    "{}/trades/{}/{}/buy-orders",
                    self.url.0, symbol, trade_request.uid
                ).as_str(),
            )
            .json(trade_request)
            .send()
            .and_then(|mut res| res.json::<TradeAcceptance>())
    }
}
