use bam::{self, config::Config, connection::Connection, json, Status};
use bam_api::header::ToBamHeader;
use comit_client::{
    rfc003, Client, ClientFactory, ClientFactoryError, SwapReject, SwapResponseError,
};
use futures::Future;
use serde_json;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};
use swap_protocols::{self, asset::Asset, SwapProtocols};
use tokio::{self, net::TcpStream};

#[derive(Debug)]
pub struct BamClient {
    comit_node_socket_addr: SocketAddr,
    bam_client: Arc<Mutex<bam::client::Client<json::Frame, json::Request, json::Response>>>,
}

impl BamClient {
    pub fn new(
        comit_node_socket_addr: SocketAddr,
        bam_client: bam::client::Client<json::Frame, json::Request, json::Response>,
    ) -> Self {
        BamClient {
            comit_node_socket_addr,
            bam_client: Arc::new(Mutex::new(bam_client)),
        }
    }
}

impl Client for BamClient {
    fn send_swap_request<
        SL: swap_protocols::rfc003::Ledger,
        TL: swap_protocols::rfc003::Ledger,
        SA: Asset,
        TA: Asset,
    >(
        &self,
        request: rfc003::Request<SL, TL, SA, TA>,
    ) -> Box<
        Future<
                Item = Result<rfc003::AcceptResponseBody<SL, TL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    > {
        let source_ledger_refund_identity = request.source_ledger_refund_identity;
        let target_ledger_success_identity = request.target_ledger_success_identity;
        let source_ledger_lock_duration = request.source_ledger_lock_duration;
        let secret_hash = request.secret_hash;

        let request = json::Request::new(
            "SWAP".into(),
            convert_args!(
                keys = String::from,
                hashmap!(
                "source_ledger" => serde_json::to_value(request.source_ledger.to_bam_header().unwrap()).unwrap(),
                "target_ledger" => serde_json::to_value(request.target_ledger.to_bam_header().unwrap()).unwrap(),
                "source_asset" => serde_json::to_value(request.source_asset.to_bam_header().unwrap()).unwrap(),
                "target_asset" => serde_json::to_value(request.target_asset.to_bam_header().unwrap()).unwrap(),
                "swap_protocol" => serde_json::to_value(SwapProtocols::Rfc003.to_bam_header().unwrap()).unwrap(),
            )
            ),
            serde_json::to_value(rfc003::RequestBody::<SL, TL> {
                source_ledger_refund_identity,
                target_ledger_success_identity,
                source_ledger_lock_duration,
                secret_hash,
            })
            .unwrap(),
        );

        debug!(
            "Making swap request to {}: {:?}",
            &self.comit_node_socket_addr, request,
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
