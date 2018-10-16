use comit_client::{Client, ClientFactory, ClientFactoryError, SwapReject, SwapResponseError};
use futures::Future;
use serde_json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};
use swap_protocols::{rfc003, wire_types};
use tokio::{self, net::TcpStream};
use transport_protocol::{self, config::Config, connection::Connection, json, Status};

#[derive(Debug)]
pub struct BamClient {
    comit_node_socket_addr: SocketAddr,
    bam_client:
        Arc<Mutex<transport_protocol::client::Client<json::Frame, json::Request, json::Response>>>,
}

impl BamClient {
    pub fn new(
        comit_node_socket_addr: SocketAddr,
        bam_client: transport_protocol::client::Client<json::Frame, json::Request, json::Response>,
    ) -> Self {
        BamClient {
            comit_node_socket_addr,
            bam_client: Arc::new(Mutex::new(bam_client)),
        }
    }
}

impl Client for BamClient {
    fn send_swap_request<
        SL: rfc003::ledger::Ledger,
        TL: rfc003::ledger::Ledger,
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

#[derive(Default, Debug)]
pub struct BamClientPool {
    clients: RwLock<HashMap<SocketAddr, Arc<BamClient>>>,
}

impl ClientFactory<BamClient> for BamClientPool {
    fn client_for(
        &self,
        comit_node_socket_addr: SocketAddr,
        //TODO: Return a future and ensure no duplicate connections
    ) -> Result<Arc<BamClient>, ClientFactoryError> {
        debug!("Trying to get client for {}", comit_node_socket_addr);
        let existing_client = self
            .clients
            .read()
            .unwrap()
            .get(&comit_node_socket_addr)
            .cloned();

        match existing_client {
            None => {
                info!(
                    "No existing connection to {}. Trying to connect.",
                    comit_node_socket_addr
                );
                let socket = TcpStream::connect(&comit_node_socket_addr).wait()?;
                info!("Connection to {} established", comit_node_socket_addr);
                let codec = json::JsonFrameCodec::default();
                let config = Config::<json::Request, json::Response>::default();
                let connection = Connection::new(config, codec, socket);
                let (connection_future, client) = connection.start::<json::JsonFrameHandler>();
                let socket_addr = comit_node_socket_addr;
                tokio::spawn(connection_future.map_err(move |e| {
                    error!(
                        "Connection to {:?} prematurely closed: {:?}",
                        socket_addr, e
                    )
                }));
                let client = Arc::new(BamClient::new(comit_node_socket_addr, client));
                let mut clients = self.clients.write().unwrap();
                clients.insert(comit_node_socket_addr, client.clone());
                debug!(
                    "Client for {} created by making a new connection",
                    comit_node_socket_addr
                );
                Ok(client)
            }
            Some(client) => {
                debug!("Retrieved existing client for {}", comit_node_socket_addr);
                Ok(client.clone())
            }
        }
    }
}
