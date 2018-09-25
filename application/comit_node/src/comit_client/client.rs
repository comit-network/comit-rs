use futures::Future;
use ganp::{ledger::Ledger, rfc003, swap};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
//use tokio::{net::TcpStream, runtime::Runtime};
use transport_protocol::{
    self,
    config::Config,
    connection::Connection,
    json,
    shutdown_handle::{self, ShutdownHandle},
    Status,
};

pub trait Client {
    fn send_swap_request<SL: Ledger, TL: Ledger, SA: Into<swap::Asset>, TA: Into<swap::Asset>>(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
                Error = transport_protocol::client::Error<json::Frame>,
            >
            + Send,
    >;
}

pub enum SwapReject {
    /// The counterparty produced an invalid response to the request
    InvalidResponse,
    /// The counterparty rejected the request
    Rejected,
    /// The counterparty had an internal error while processing the request
    InternalError,
}

pub struct DefaultClient {
    comit_node_socket_addr: SocketAddr,
    bam_client:
        Arc<Mutex<transport_protocol::client::Client<json::Frame, json::Request, json::Response>>>,
}

impl DefaultClient {
    pub fn new(
        comit_node_socket_addr: SocketAddr,
        bam_client: Arc<
            Mutex<transport_protocol::client::Client<json::Frame, json::Request, json::Response>>,
        >,
    ) -> Self {
        DefaultClient {
            comit_node_socket_addr,
            bam_client,
        }
    }
}

impl Client for DefaultClient {
    fn send_swap_request<SL: Ledger, TL: Ledger, SA: Into<swap::Asset>, TA: Into<swap::Asset>>(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponse<SL, TL>, SwapReject>,
                Error = transport_protocol::client::Error<json::Frame>,
            >
            + Send,
    > {
        unimplemented!()

        // let (headers, body) = request.into_headers_and_body();
        // let request = json::Request::from_headers_and_body("SWAP".into(), headers, body).expect("Serialization of this should never fail");

        // debug!(
        //     "Making swap request to {}: {:?}",
        //     &self.comit_node_socket_addr, request
        // );

        // let response = self.bam_client.send_request(request).map(|response|{
        //     match response.status() {
        //         Status::OK(_) => {
        //             info!(
        //                 "{} accepted swap request: {:?}",
        //                 &self.comit_node_socket_addr, response
        //             );
        //             Ok(serde_json::from_value(response.body().clone())
        //                .map_err(SwapReject::InvalidResponse)?)
        //         }
        //         Status::SE(_) => {
        //             info!(
        //                 "{} rejected swap request: {:?}",
        //                 &self.comit_node_socket_addr, response
        //             );
        //             Err(SwapReject::Rejected)
        //         }
        //         Status::RE(_) => {
        //             error!(
        //                 "{} rejected swap request because of an internal error: {:?}",
        //                 &self.comit_node_socket_addr, response
        //             );
        //             Err(SwapReject::InternalError)
        //         }
        //     }
        // });

        // Box::new(response)
    }
}
