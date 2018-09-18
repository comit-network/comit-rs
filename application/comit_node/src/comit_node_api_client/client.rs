use common_types::{secret::SecretHash, TradingSymbol};
use futures::Future;
use ganp::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    rfc003, swap,
};
use serde_json;
use std::{io, net::SocketAddr};
use swaps::common::TradeId;
use tokio::{net::TcpStream, runtime::Runtime};
use transport_protocol::{
    client::{self, Client},
    config::Config,
    connection::Connection,
    json,
    shutdown_handle::{self, ShutdownHandle},
    Status,
};

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
            secret_hash: self.contract_secret_lock,
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
        trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, SwapRequestError>;
    fn create_sell_order(
        &self,
        trade_request: &OrderRequestBody<Bitcoin, Ethereum>,
    ) -> Result<OrderResponseBody<Bitcoin, Ethereum>, SwapRequestError>;
}

#[derive(Debug)]
pub enum SwapRequestError {
    /// The other node received our request but rejected it
    Rejected,
    /// The connection failed to open
    FailedToConnect(io::Error),
    /// The other node had an internal error while processing our request
    ReceiverError,
    /// A JSON error occurred during serialization of request
    JsonSer(serde_json::Error),
    /// A JSON error occurred during deserialization of response
    JsonDe(serde_json::Error),
    /// The transport layer had a problem sending the frame or receiving a response frame
    ClientError(client::Error<json::Frame>),
}

impl From<client::Error<json::Frame>> for SwapRequestError {
    fn from(e: client::Error<json::Frame>) -> Self {
        SwapRequestError::ClientError(e)
    }
}

pub struct DefaultApiClient {
    comit_node_socket_addr: SocketAddr,
}

impl DefaultApiClient {
    pub fn new(comit_node_socket_addr: SocketAddr) -> Self {
        DefaultApiClient {
            comit_node_socket_addr,
        }
    }

    fn connect_to_comit_node(
        &self,
        runtime: &mut Runtime,
    ) -> (Result<
        (
            Client<json::Frame, json::Request, json::Response>,
            ShutdownHandle,
        ),
        io::Error,
    >) {
        info!(
            "Connecting to {} to make request",
            &self.comit_node_socket_addr
        );
        let socket = TcpStream::connect(&self.comit_node_socket_addr).wait()?;
        let codec = json::JsonFrameCodec::default();
        let config: Config<json::Request, json::Response> = Config::new();
        let connection = Connection::new(config, codec, socket);

        let (connection_future, client) = connection.start::<json::JsonFrameHandler>();
        let (connection_future, shutdown_handle) = shutdown_handle::new(connection_future);
        let socket_addr = self.comit_node_socket_addr.clone();

        runtime.spawn(connection_future.map_err(move |e| {
            error!(
                "Connection to {:?} prematurely closed: {:?}",
                socket_addr, e
            )
        }));

        Ok((client, shutdown_handle))
    }

    fn send_swap_request<SL: Ledger, TL: Ledger, SA: Into<swap::Asset>, TA: Into<swap::Asset>>(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Result<rfc003::AcceptResponse<SL, TL>, SwapRequestError> {
        let mut runtime = Runtime::new().expect("creating a tokio runtime should never fail");

        let (mut client, _shutdown_handle) = self
            .connect_to_comit_node(&mut runtime)
            .map_err(SwapRequestError::FailedToConnect)?;

        let (headers, body) = request.into_headers_and_body();
        let request = json::Request::from_headers_and_body("SWAP".into(), headers, body)
            .map_err(SwapRequestError::JsonSer)?;

        debug!(
            "Making swap request to {}: {:?}",
            &self.comit_node_socket_addr, request
        );

        let response = client.send_request(request).wait()?;

        match response.status() {
            Status::OK(_) => {
                info!(
                    "{} accepted swap request: {:?}",
                    &self.comit_node_socket_addr, response
                );
                Ok(serde_json::from_value(response.body().clone())
                    .map_err(SwapRequestError::JsonDe)?)
            }
            Status::SE(_) => {
                info!(
                    "{} rejected swap request: {:?}",
                    &self.comit_node_socket_addr, response
                );
                Err(SwapRequestError::Rejected)
            }
            Status::RE(_) => {
                error!(
                    "{} rejected swap request because of an internal error: {:?}",
                    &self.comit_node_socket_addr, response
                );
                Err(SwapRequestError::ReceiverError)
            }
        }
    }
}

impl ApiClient for DefaultApiClient {
    fn create_buy_order(
        &self,
        trade_request: &OrderRequestBody<Ethereum, Bitcoin>,
    ) -> Result<OrderResponseBody<Ethereum, Bitcoin>, SwapRequestError> {
        self.send_swap_request(trade_request.clone().into())
            .map(|x| x.into())
    }

    fn create_sell_order(
        &self,
        trade_request: &OrderRequestBody<Bitcoin, Ethereum>,
    ) -> Result<OrderResponseBody<Bitcoin, Ethereum>, SwapRequestError> {
        self.send_swap_request(trade_request.clone().into())
            .map(|x| x.into())
    }
}
