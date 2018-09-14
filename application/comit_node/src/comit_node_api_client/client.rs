use common_types::{secret::SecretHash, TradingSymbol};
use futures::Future;
use ganp::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    rfc003, swap,
};
use reqwest;
use serde_json::{self, Value as JsonValue};
use std::{collections::HashMap, net::SocketAddr, sync::Mutex};
use swaps::common::TradeId;
use tokio::{self, net::TcpStream, runtime::Runtime};
use transport_protocol::{client::Client, config::Config, connection::Connection, json, Status};

#[derive(Clone)]
pub struct ComitNodeUrl(pub String);

#[derive(Serialize, Deserialize)]
struct OfferRequestBody {
    amount: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody<Buy: Ledger, Sell: Ledger> {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: Buy::Quantity,
    pub sell_amount: Sell::Quantity,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody<Buy: Ledger, Sell: Ledger> {
    pub contract_secret_lock: SecretHash,
    pub alice_refund_address: Sell::Address,
    pub alice_success_address: Buy::Address,
    pub alice_contract_time_lock: Sell::LockDuration,
    pub buy_amount: Buy::Quantity,
    pub sell_amount: Sell::Quantity,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderResponseBody<Buy: Ledger, Sell: Ledger> {
    pub bob_refund_address: Buy::Address,
    pub bob_contract_time_lock: Buy::LockDuration,
    pub bob_success_address: Sell::Address,
}

impl<SL: Ledger, TL: Ledger> Into<rfc003::Request<SL, TL, SL::Quantity, TL::Quantity>>
    for OrderRequestBody<TL, SL>
{
    fn into(self) -> rfc003::Request<SL, TL, SL::Quantity, TL::Quantity> {
        rfc003::Request {
            source_asset: self.sell_amount,
            target_asset: self.buy_amount,
            source_ledger: SL::default(),
            target_ledger: TL::default(),
            source_ledger_refund_identity: self.alice_refund_address.into(),
            target_ledger_success_identity: self.alice_success_address.into(),
            source_ledger_lock_duration: self.alice_contract_time_lock,
            secret_hash: self.contract_secret_lock.to_string(),
        }
    }
}

impl<SL: Ledger, TL: Ledger> From<rfc003::AcceptResponse<SL, TL>> for OrderResponseBody<TL, SL> {
    fn from(accept_response: rfc003::AcceptResponse<SL, TL>) -> Self {
        OrderResponseBody {
            bob_refund_address: accept_response.target_ledger_refund_identity.into(),
            bob_success_address: accept_response.source_ledger_success_identity.into(),
            bob_contract_time_lock: accept_response.target_ledger_lock_duration,
        }
    }
}

pub trait ApiClient: Send + Sync {
    fn create_buy_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, reqwest::Error>;
    fn create_sell_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Bitcoin, Ethereum>,
    ) -> Result<OrderResponseBody<Bitcoin, Ethereum>, reqwest::Error>;
}

pub struct DefaultApiClient {
    client: reqwest::Client,
    url: ComitNodeUrl,
    ganp_socket_addr: SocketAddr,
}

#[derive(Debug)]
enum SwapRequestError {
    Rejected,
    FailedToConnect,
    Json(serde_json::Error),
}

impl From<serde_json::Error> for SwapRequestError {
    fn from(e: serde_json::Error) -> Self {
        SwapRequestError::Json(e)
    }
}

impl DefaultApiClient {
    pub fn new(url: ComitNodeUrl, ganp_socket_addr: SocketAddr) -> Self {
        DefaultApiClient {
            url,
            client: reqwest::Client::new(),
            ganp_socket_addr,
        }
    }

    fn send_swap_request<SL: Ledger, TL: Ledger, SA: Into<swap::Asset>, TA: Into<swap::Asset>>(
        &self,
        message: rfc003::Request<SL, TL, SA, TA>,
    ) -> Result<rfc003::AcceptResponse<SL, TL>, SwapRequestError> {
        debug!("connecting to {} to make request", &self.ganp_socket_addr);
        let socket = match TcpStream::connect(&self.ganp_socket_addr).wait() {
            Ok(socket) => socket,
            Err(e) => return Err(SwapRequestError::FailedToConnect),
        };

        let codec = json::JsonFrameCodec::default();
        let config: Config<json::Request, json::Response> = Config::new();
        let connection = Connection::new(config, codec, socket);
        let (headers, body) = message.into_headers_and_body();

        let headers_json = serde_json::to_value(headers).unwrap();
        let mut headers_hashmap = HashMap::new();

        match headers_json {
            JsonValue::Object(map) => {
                for (k, v) in map {
                    headers_hashmap.insert(k, v);
                }
            }
            _ => unreachable!(),
        }

        let request = json::Request::new(
            "SWAP".into(),
            headers_hashmap,
            serde_json::to_value(body).unwrap(),
        );

        let (connection_future, mut client) = connection.start::<json::JsonFrameHandler>();
        let socket_addr = self.ganp_socket_addr.clone();

        let mut runtime = Runtime::new().unwrap();

        runtime.spawn(
            connection_future
                .map_err(move |e| error!("connection to {:?} closed: {:?}", socket_addr, e)),
        );

        let response = match client.send_request(request).wait() {
            Ok(response) => response,
            Err(e) => panic!("request failed!: {:?}", e),
        };

        match response.status() {
            Status::OK(_) => {
                info!("GOT RESPONSE: {:?}", response);
                Ok(serde_json::from_value(response.body().clone())?)
            }
            _ => {
                error!(
                    "{} rejected the swap request with: {:?}",
                    &self.ganp_socket_addr, response
                );
                Err(SwapRequestError::Rejected)
            }
        }
    }
}

impl ApiClient for DefaultApiClient {
    fn create_buy_order(
        &self,
        symbol: TradingSymbol,
        uid: TradeId,
        trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, reqwest::Error> {
        match self.send_swap_request(trade_request.clone().into()) {
            Ok(rfc_accept) => Ok(rfc_accept.into()),
            Err(e) => {
                match e {
                    SwapRequestError::FailedToConnect => {
                        error!("Failed to connect to: {}", &self.ganp_socket_addr)
                    }
                    e => error!("something bad: {:?}", e),
                }
                panic!("request failed");
            }
        }
        // self.client
        //     .post(format!("{}/trades/{}/{}/buy-orders", self.url.0, symbol, uid).as_str())
        //     .json(trade_request)
        //     .send()
        //     .and_then(|mut res| res.json::<OrderResponseBody<Ethereum, Bitcoin>>())
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
