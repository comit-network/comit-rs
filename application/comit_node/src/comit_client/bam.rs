use crate::{
    bam_ext::{FromBamHeader, ToBamHeader},
    comit_client::{
        rfc003, Client, ClientFactory, ClientFactoryError, ClientPool, SwapDeclineReason,
        SwapReject, SwapResponseError,
    },
    swap_protocols::{self, asset::Asset, SwapProtocols},
};
use bam::{self, config::Config, connection, json, Status};
use futures::Future;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};
use tokio::{self, net::TcpStream};

#[derive(Debug)]
pub struct BamClient {
    comit_node_socket_addr: SocketAddr,
    bam_client: Arc<Mutex<bam::client::Client<json::Frame, json::OutgoingRequest, json::Response>>>,
}

impl BamClient {
    pub fn new(
        comit_node_socket_addr: SocketAddr,
        bam_client: bam::client::Client<json::Frame, json::OutgoingRequest, json::Response>,
    ) -> Self {
        BamClient {
            comit_node_socket_addr,
            bam_client: Arc::new(Mutex::new(bam_client)),
        }
    }

    fn build_swap_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<json::OutgoingRequest, serde_json::Error> {
        let alpha_ledger_refund_identity = request.alpha_ledger_refund_identity;
        let beta_ledger_redeem_identity = request.beta_ledger_redeem_identity;
        let alpha_expiry = request.alpha_expiry;
        let beta_expiry = request.beta_expiry;
        let secret_hash = request.secret_hash;

        Ok(json::OutgoingRequest::new("SWAP")
            .with_header("alpha_ledger", request.alpha_ledger.into().to_bam_header()?)
            .with_header("beta_ledger", request.beta_ledger.into().to_bam_header()?)
            .with_header("alpha_asset", request.alpha_asset.into().to_bam_header()?)
            .with_header("beta_asset", request.beta_asset.into().to_bam_header()?)
            .with_header("swap_protocol", SwapProtocols::Rfc003.to_bam_header()?)
            .with_body(serde_json::to_value(rfc003::RequestBody::<AL, BL> {
                alpha_ledger_refund_identity,
                beta_ledger_redeem_identity,
                alpha_expiry,
                beta_expiry,
                secret_hash,
            })?))
    }
}

impl Client for BamClient {
    fn send_swap_request<
        AL: swap_protocols::rfc003::Ledger,
        BL: swap_protocols::rfc003::Ledger,
        AA: Asset,
        BA: Asset,
    >(
        &self,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Box<
        dyn Future<
                Item = Result<rfc003::AcceptResponseBody<AL, BL>, SwapReject>,
                Error = SwapResponseError,
            > + Send,
    > {
        let request = Self::build_swap_request(request)
            .expect("constructing a bam::json::OutoingRequest should never fail!");

        debug!(
            "Making swap request to {}: {:?}",
            &self.comit_node_socket_addr, request,
        );
        let mut bam_client = self.bam_client.lock().unwrap();

        let socket_addr = self.comit_node_socket_addr;

        let response = bam_client
            .send_request(request)
            .then(move |result| match result {
                Ok(mut response) => match response.status() {
                    Status::OK(_) => {
                        info!("{} accepted swap request: {:?}", socket_addr, response);
                        match serde_json::from_value(response.body().clone()) {
                            Ok(response) => Ok(Ok(response)),
                            Err(_e) => Err(SwapResponseError::InvalidResponse),
                        }
                    }
                    Status::SE(20) => {
                        info!("{} declined swap request: {:?}", socket_addr, response);
                        Ok(Err({
                            let reason = response
                                .take_header("REASON")
                                .map(SwapDeclineReason::from_bam_header)
                                .map_or(Ok(None), |x| x.map(Some))
                                .map_err(|e| {
                                    error!(
                                        "Could not deserialize header in response {:?}: {}",
                                        response, e,
                                    );
                                    SwapResponseError::InvalidResponse
                                })?;

                            SwapReject::Declined { reason }
                        }))
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
        // TODO: Return a future and ensure no duplicate connections
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

                let response_source = Arc::new(Mutex::new(json::JsonResponseSource::default()));
                let incoming_frames = json::JsonFrameHandler::create(
                    Config::<json::ValidatedIncomingRequest, json::Response>::default(),
                    Arc::clone(&response_source),
                );
                let (client, outgoing_frames) = bam::client::Client::create(response_source);
                let connection = connection::new(codec, socket, incoming_frames, outgoing_frames);
                let socket_addr = comit_node_socket_addr;
                tokio::spawn(connection.map_err(move |e| {
                    error!(
                        "Connection to {:?} prematurely closed: {:?}",
                        socket_addr, e
                    )
                }));
                let client = Arc::new(BamClient::new(comit_node_socket_addr, client));
                let mut clients = self.clients.write().unwrap();
                clients.insert(comit_node_socket_addr, Arc::clone(&client));
                debug!(
                    "Client for {} created by making a new connection",
                    comit_node_socket_addr
                );
                Ok(client)
            }
            Some(client) => {
                debug!("Retrieved existing client for {}", comit_node_socket_addr);
                Ok(Arc::clone(&client))
            }
        }
    }
    fn add_client(&self, comit_node_socket_addr: SocketAddr, client: Arc<BamClient>) {
        debug!("Adding {:?} to list of peers", comit_node_socket_addr);
        let mut clients = self.clients.write().unwrap();

        clients.insert(comit_node_socket_addr, client);
    }
}

impl ClientPool for BamClientPool {
    fn connected_addrs(&self) -> Vec<SocketAddr> {
        let clients = self.clients.read().unwrap();

        let mut keys = Vec::new();
        for key in clients.keys() {
            keys.push(key.clone());
        }
        keys
    }
}
