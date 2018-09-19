use bitcoin_support::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use futures::Future;
use ganp::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    rfc003, swap,
};
use serde_json;
use std::{io, net::SocketAddr};
use tokio::{net::TcpStream, runtime::Runtime};
use transport_protocol::{
    client::{self, Client},
    config::Config,
    connection::Connection,
    json,
    shutdown_handle::{self, ShutdownHandle},
    Status,
};

pub trait ApiClient: Send + Sync {
    fn create_buy_order(
        &self,
        swap_request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> Result<rfc003::AcceptResponse<Bitcoin, Ethereum>, SwapRequestError>;
    fn create_sell_order(
        &self,
        swap_request: rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
    ) -> Result<rfc003::AcceptResponse<Ethereum, Bitcoin>, SwapRequestError>;
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
        swap_request: rfc003::Request<Bitcoin, Ethereum, BitcoinQuantity, EthereumQuantity>,
    ) -> Result<rfc003::AcceptResponse<Bitcoin, Ethereum>, SwapRequestError> {
        self.send_swap_request(swap_request)
    }

    fn create_sell_order(
        &self,
        swap_request: rfc003::Request<Ethereum, Bitcoin, EthereumQuantity, BitcoinQuantity>,
    ) -> Result<rfc003::AcceptResponse<Ethereum, Bitcoin>, SwapRequestError> {
        self.send_swap_request(swap_request)
    }
}
