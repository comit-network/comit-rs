use futures::Future;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use swap_protocols::{ledger::Ledger, rfc003, wire_types};
//use tokio::{net::TcpStream, runtime::Runtime};
use serde_json;
use transport_protocol::{self, json, Status};

pub trait Client {
    fn send_swap_request<
        SL: Ledger,
        TL: Ledger,
        SA: Into<wire_types::Asset>,
        TA: Into<wire_types::Asset>,
    >(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    >;
}

#[derive(Clone, Debug)]
pub enum SwapReject {
    /// The counterparty rejected the request
    Rejected,
}

#[derive(Debug)]
pub enum SwapResponseError {
    /// The counterparty had an internal error while processing the request
    InternalError,
    /// The counterparty produced a response that caused at the transport level
    TransportError,
    /// The counterparty produced an invalid response to the request
    InvalidResponse,
}

#[derive(Debug)]
pub struct DefaultClient {
    comit_node_socket_addr: SocketAddr,
    bam_client:
        Arc<Mutex<transport_protocol::client::Client<json::Frame, json::Request, json::Response>>>,
}

impl DefaultClient {
    pub fn new(
        comit_node_socket_addr: SocketAddr,
        bam_client: transport_protocol::client::Client<json::Frame, json::Request, json::Response>,
    ) -> Self {
        DefaultClient {
            comit_node_socket_addr,
            bam_client: Arc::new(Mutex::new(bam_client)),
        }
    }
}

impl Client for DefaultClient {
    fn send_swap_request<
        SL: Ledger,
        TL: Ledger,
        SA: Into<wire_types::Asset>,
        TA: Into<wire_types::Asset>,
    >(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    > {
        let (headers, body) = request.into_headers_and_body();
        let request = json::Request::from_headers_and_body("SWAP".into(), headers, body)
            .expect("Serialization of this should never fail");

        debug!(
            "Making swap request to {}: {:?}",
            &self.comit_node_socket_addr, request
        );
        let mut bam_client = self.bam_client.lock().unwrap();

        let socket_addr = self.comit_node_socket_addr;

        let response = bam_client
            .send_request(request)
            .then(move |result| match result {
                Ok(response) => match response.status() {
                    Status::OK(_) => {
                        info!("{} accepted swap request: {:?}", socket_addr, response);
                        match serde_json::from_value(response.body().clone()) {
                            Ok(response) => Ok(Ok(response)),
                            Err(_e) => Err(SwapResponseError::InvalidResponse),
                        }
                    }
                    Status::SE(_) => {
                        info!("{} rejected swap request: {:?}", socket_addr, response);
                        Ok(Err(SwapReject::Rejected))
                    }
                    Status::RE(_) => {
                        error!(
                            "{} rejected swap request because of an internal error: {:?}",
                            socket_addr, response
                        );
                        Err(SwapResponseError::InternalError)
                    }
                },
                Err(transport_error) => {
                    error!(
                        "transport error during request to {:?}:{:?}",
                        socket_addr, transport_error
                    );
                    Err(SwapResponseError::TransportError)
                }
            });

        Box::new(response)
    }
}
