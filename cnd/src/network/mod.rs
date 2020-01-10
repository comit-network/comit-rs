pub mod transport;

pub use transport::ComitTransport;

use crate::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    config::Settings,
    db::{Save, Sqlite, Swap},
    libp2p_comit_ext::{FromHeader, ToHeader},
    seed::{DeriveSwapSeed, RootSeed},
    swap_protocols::{
        asset::{Asset, AssetKind},
        rfc003::{
            self, bob,
            messages::{Decision, DeclineResponseBody, Request, SwapDeclineReason},
            state_store::{InMemoryStateStore, StateStore},
            Ledger,
        },
        HashFunction, LedgerKind, Role, SwapId, SwapProtocol,
    },
};
use async_trait::async_trait;
use futures::{
    future::Future,
    stream::Stream,
    sync::oneshot::{self, Sender},
};
use futures_core::{compat::Future01CompatExt, FutureExt, TryFutureExt};
use libp2p::{
    core::muxing::SubstreamRef,
    identity::{self, ed25519},
    mdns::Mdns,
    swarm::{
        protocols_handler::DummyProtocolsHandler, ExpandedSwarm, IntoProtocolsHandlerSelect,
        NetworkBehaviourEventProcess,
    },
    Multiaddr, NetworkBehaviour, PeerId,
};
use libp2p_comit::{
    frame::{self, CodecError, OutboundRequest, Response, ValidatedInboundRequest},
    handler::{ComitHandler, ProtocolInEvent, ProtocolOutEvent},
    BehaviourOutEvent, Comit, PendingInboundRequest,
};
use libp2p_core::{
    either::{EitherError, EitherOutput},
    muxing::StreamMuxerBox,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    io,
    sync::{Arc, Mutex},
};
use tokio_compat::runtime::{Runtime, TaskExecutor};

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
#[allow(clippy::type_complexity)]
pub struct Swarm {
    #[derivative(Debug = "ignore")]
    swarm: Arc<
        Mutex<
            ExpandedSwarm<
                ComitTransport,
                ComitNode<SubstreamRef<Arc<StreamMuxerBox>>>,
                EitherOutput<ProtocolInEvent, void::Void>,
                EitherOutput<ProtocolOutEvent, void::Void>,
                IntoProtocolsHandlerSelect<
                    ComitHandler<SubstreamRef<Arc<StreamMuxerBox>>>,
                    DummyProtocolsHandler<SubstreamRef<Arc<StreamMuxerBox>>>,
                >,
                EitherError<CodecError, void::Void>,
            >,
        >,
    >,
    local_peer_id: PeerId,
}

impl Swarm {
    pub fn new(
        settings: &Settings,
        seed: RootSeed,
        runtime: &mut Runtime,
        bitcoin_connector: &BitcoindConnector,
        ethereum_connector: &Web3Connector,
        state_store: &Arc<InMemoryStateStore>,
        database: &Sqlite,
    ) -> anyhow::Result<Self> {
        let local_key_pair = derive_key_pair(&seed);
        let local_peer_id = PeerId::from(local_key_pair.clone().public());
        log::info!("Starting with peer_id: {}", local_peer_id);

        let transport = transport::build_comit_transport(local_key_pair);
        let behaviour = ComitNode::new(
            bitcoin_connector.clone(),
            ethereum_connector.clone(),
            Arc::clone(&state_store),
            seed,
            database.clone(),
            runtime.executor(),
        )?;
        let mut swarm = libp2p::Swarm::new(transport, behaviour, local_peer_id.clone());

        for addr in settings.network.listen.clone() {
            libp2p::Swarm::listen_on(&mut swarm, addr)
                .expect("Could not listen on specified address");
        }

        let swarm = Arc::new(Mutex::new(swarm));

        let swarm_worker = futures::stream::poll_fn({
            let swarm = swarm.clone();
            move || swarm.lock().unwrap().poll()
        })
        .for_each(|_| Ok(()))
        .map_err(|e| {
            log::error!("failed with {:?}", e);
        });

        runtime.spawn(swarm_worker);

        Ok(Self {
            swarm,
            local_peer_id,
        })
    }
}

fn derive_key_pair(seed: &RootSeed) -> identity::Keypair {
    let bytes = seed.sha256_with_seed(&[b"NODE_ID"]);
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
    identity::Keypair::Ed25519(key.into())
}

#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode<TSubstream> {
    comit: Comit<TSubstream>,
    mdns: Mdns<TSubstream>,

    #[behaviour(ignore)]
    pub bitcoin_connector: BitcoindConnector,
    #[behaviour(ignore)]
    pub ethereum_connector: Web3Connector,
    #[behaviour(ignore)]
    pub state_store: Arc<InMemoryStateStore>,
    #[behaviour(ignore)]
    pub seed: RootSeed,
    #[behaviour(ignore)]
    pub db: Sqlite,
    #[behaviour(ignore)]
    response_channels: Arc<Mutex<HashMap<SwapId, oneshot::Sender<Response>>>>,
    #[behaviour(ignore)]
    task_executor: TaskExecutor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialInformation {
    pub peer_id: PeerId,
    pub address_hint: Option<Multiaddr>,
}

impl Display for DialInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.address_hint {
            None => write!(f, "{}", self.peer_id),
            Some(address_hint) => write!(f, "{}@{}", self.peer_id, address_hint),
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum RequestError {
    #[error("peer node had an internal error while processing the request")]
    InternalError,
    #[error("peer node produced an invalid response")]
    InvalidResponse,
    #[error("failed to establish a new connection to make the request")]
    Connecting(io::ErrorKind),
    #[error("unable to send the data on the existing connection")]
    Connection,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Reason {
    pub value: SwapDeclineReason,
}

impl<TSubstream> ComitNode<TSubstream> {
    pub fn new(
        bitcoin_connector: BitcoindConnector,
        ethereum_connector: Web3Connector,
        state_store: Arc<InMemoryStateStore>,
        seed: RootSeed,
        db: Sqlite,
        task_executor: TaskExecutor,
    ) -> Result<Self, io::Error> {
        let mut swap_headers = HashSet::new();
        swap_headers.insert("id".into());
        swap_headers.insert("alpha_ledger".into());
        swap_headers.insert("beta_ledger".into());
        swap_headers.insert("alpha_asset".into());
        swap_headers.insert("beta_asset".into());
        swap_headers.insert("protocol".into());

        let mut known_headers = HashMap::new();
        known_headers.insert("SWAP".into(), swap_headers);

        Ok(Self {
            comit: Comit::new(known_headers),
            mdns: Mdns::new()?,
            bitcoin_connector,
            ethereum_connector,
            state_store,
            seed,
            db,
            response_channels: Arc::new(Mutex::new(HashMap::new())),
            task_executor,
        })
    }

    pub fn send_request(
        &mut self,
        peer_id: DialInformation,
        request: OutboundRequest,
    ) -> Box<dyn Future<Item = Response, Error = ()> + Send> {
        self.comit
            .send_request((peer_id.peer_id, peer_id.address_hint), request)
    }
}

async fn handle_request(
    db: Sqlite,
    seed: RootSeed,
    state_store: Arc<InMemoryStateStore>,
    counterparty: PeerId,
    mut request: ValidatedInboundRequest,
) -> Result<SwapId, Response> {
    match request.request_type() {
        "SWAP" => {
            let protocol: SwapProtocol = header!(request
                .take_header("protocol")
                .map(SwapProtocol::from_header));
            match protocol {
                SwapProtocol::Rfc003(hash_function) => {
                    let swap_id = header!(request.take_header("id").map(SwapId::from_header));
                    let alpha_ledger = header!(request
                        .take_header("alpha_ledger")
                        .map(LedgerKind::from_header));
                    let beta_ledger = header!(request
                        .take_header("beta_ledger")
                        .map(LedgerKind::from_header));
                    let alpha_asset = header!(request
                        .take_header("alpha_asset")
                        .map(AssetKind::from_header));
                    let beta_asset = header!(request
                        .take_header("beta_asset")
                        .map(AssetKind::from_header));

                    match (alpha_ledger, beta_ledger, alpha_asset, beta_asset) {
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Ether(beta_asset),
                        ) => {
                            let request = rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            );
                            insert_state_for_bob(
                                db.clone(),
                                seed,
                                state_store.clone(),
                                counterparty,
                                request,
                            )
                            .await
                            .expect("Could not save state to db");
                            Ok(swap_id)
                        }
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Ether(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => {
                            let request = rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            );
                            insert_state_for_bob(
                                db.clone(),
                                seed,
                                state_store.clone(),
                                counterparty,
                                request,
                            )
                            .await
                            .expect("Could not save state to db");
                            Ok(swap_id)
                        }
                        (
                            LedgerKind::Bitcoin(alpha_ledger),
                            LedgerKind::Ethereum(beta_ledger),
                            AssetKind::Bitcoin(alpha_asset),
                            AssetKind::Erc20(beta_asset),
                        ) => {
                            let request = rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            );
                            insert_state_for_bob(
                                db.clone(),
                                seed,
                                state_store.clone(),
                                counterparty,
                                request,
                            )
                            .await
                            .expect("Could not save state to db");

                            Ok(swap_id)
                        }
                        (
                            LedgerKind::Ethereum(alpha_ledger),
                            LedgerKind::Bitcoin(beta_ledger),
                            AssetKind::Erc20(alpha_asset),
                            AssetKind::Bitcoin(beta_asset),
                        ) => {
                            let request = rfc003_swap_request(
                                swap_id,
                                alpha_ledger,
                                beta_ledger,
                                alpha_asset,
                                beta_asset,
                                hash_function,
                                body!(request.take_body_as()),
                            );
                            insert_state_for_bob(
                                db.clone(),
                                seed,
                                state_store.clone(),
                                counterparty,
                                request,
                            )
                            .await
                            .expect("Could not save state to db");
                            Ok(swap_id)
                        }
                        (alpha_ledger, beta_ledger, alpha_asset, beta_asset) => {
                            log::warn!(
                                    "swapping {:?} to {:?} from {:?} to {:?} is currently not supported", alpha_asset, beta_asset, alpha_ledger, beta_ledger
                                );

                            let decline_body = DeclineResponseBody {
                                reason: Some(SwapDeclineReason::UnsupportedSwap),
                            };

                            Err(Response::empty()
                                .with_header(
                                    "decision",
                                    Decision::Declined
                                        .to_header()
                                        .expect("Decision should not fail to serialize"),
                                )
                                .with_body(serde_json::to_value(decline_body).expect(
                                    "decline body should always serialize into serde_json::Value",
                                )))
                        }
                    }
                }
            }
        }

        // This case is just catered for, because of rust. It can only happen
        // if there is a typo in the request_type within the program. The request
        // type is checked on the messaging layer and will be handled there if
        // an unknown request_type is passed in.
        request_type => {
            log::warn!("request type '{}' is unknown", request_type);

            Err(Response::empty().with_header(
                "decision",
                Decision::Declined
                    .to_header()
                    .expect("Decision should not fail to serialize"),
            ))
        }
    }
}

#[allow(clippy::type_complexity)]
async fn insert_state_for_bob<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, DB>(
    db: DB,
    seed: RootSeed,
    state_store: Arc<InMemoryStateStore>,
    counterparty: PeerId,
    swap_request: Request<AL, BL, AA, BA>,
) -> anyhow::Result<()>
where
    DB: Save<Request<AL, BL, AA, BA>> + Save<Swap>,
{
    let id = swap_request.swap_id;
    let seed = seed.derive_swap_seed(id);

    Save::save(&db, Swap::new(id, Role::Bob, counterparty)).await?;
    Save::save(&db, swap_request.clone()).await?;

    let state = bob::State::proposed(swap_request.clone(), seed);
    state_store.insert(id, state);

    Ok(())
}

/// Get the `PeerId` of this node.
pub trait LocalPeerId {
    fn local_peer_id(&self) -> PeerId;
}

impl LocalPeerId for Swarm {
    fn local_peer_id(&self) -> PeerId {
        self.local_peer_id.clone()
    }
}

/// Get `PeerId`s of connected nodes.
#[async_trait]
pub trait ComitPeers {
    async fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static>;
}

#[async_trait]
impl ComitPeers for Swarm {
    async fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static> {
        let mut swarm = self.swarm.lock().unwrap();
        Box::new(swarm.comit.connected_peers())
    }
}

/// IP addresses local node is listening on.
#[async_trait]
pub trait ListenAddresses {
    async fn listen_addresses(&self) -> Vec<Multiaddr>;
}

#[async_trait]
impl ListenAddresses for Swarm {
    async fn listen_addresses(&self) -> Vec<Multiaddr> {
        let swarm = self.swarm.lock().unwrap();

        libp2p::Swarm::listeners(&swarm)
            .chain(libp2p::Swarm::external_addresses(&swarm))
            .cloned()
            .collect()
    }
}

/// Get pending network requests for swap.
#[async_trait]
pub trait PendingRequestFor {
    async fn pending_request_for(&self, swap: SwapId) -> Option<oneshot::Sender<Response>>;
}

#[async_trait]
impl PendingRequestFor for Swarm {
    async fn pending_request_for(&self, swap: SwapId) -> Option<Sender<Response>> {
        let swarm = self.swarm.lock().unwrap();
        let mut response_channels = swarm.response_channels.lock().unwrap();
        response_channels.remove(&swap)
    }
}

/// Send swap request to connected peer.
#[async_trait]
pub trait SendRequest {
    async fn send_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
        &self,
        peer_identity: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<rfc003::Response<AL, BL>, RequestError>;
}

#[async_trait]
impl SendRequest for Swarm {
    async fn send_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
        &self,
        dial_information: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<rfc003::Response<AL, BL>, RequestError> {
        let id = request.swap_id;
        let request = build_outbound_request(request)
            .expect("constructing a frame::OutoingRequest should never fail!");

        let result = {
            let mut guard = self.swarm.lock().unwrap();
            let swarm = &mut *guard;

            log::debug!(
                "Making swap request to {}: {:?}",
                dial_information.clone(),
                id,
            );

            swarm
                .send_request(dial_information.clone(), request)
                .compat()
        }
        .await;

        match result {
            Ok(mut response) => {
                let decision = response
                    .take_header("decision")
                    .map(Decision::from_header)
                    .map_or(Ok(None), |x| x.map(Some))
                    .map_err(|e| {
                        log::error!(
                            "Could not deserialize header in response {:?}: {}",
                            response,
                            e,
                        );
                        RequestError::InvalidResponse
                    })?;

                match decision {
                    Some(Decision::Accepted) => {
                        match serde_json::from_value::<rfc003::messages::AcceptResponseBody<AL, BL>>(
                            response.body().clone(),
                        ) {
                            Ok(body) => Ok(Ok(rfc003::Accept {
                                swap_id: id,
                                beta_ledger_refund_identity: body.beta_ledger_refund_identity,
                                alpha_ledger_redeem_identity: body.alpha_ledger_redeem_identity,
                            })),
                            Err(_e) => Err(RequestError::InvalidResponse),
                        }
                    }

                    Some(Decision::Declined) => {
                        match serde_json::from_value::<rfc003::messages::DeclineResponseBody>(
                            response.body().clone(),
                        ) {
                            Ok(body) => Ok(Err(rfc003::Decline {
                                swap_id: id,
                                reason: body.reason,
                            })),
                            Err(_e) => Err(RequestError::InvalidResponse),
                        }
                    }

                    None => Err(RequestError::InvalidResponse),
                }
            }
            Err(e) => {
                log::error!(
                    "Unable to request over connection {:?}:{:?}",
                    dial_information,
                    e
                );
                Err(RequestError::Connection)
            }
        }
    }
}

impl<TSubstream> NetworkBehaviourEventProcess<BehaviourOutEvent> for ComitNode<TSubstream> {
    fn inject_event(&mut self, event: BehaviourOutEvent) {
        match event {
            BehaviourOutEvent::PendingInboundRequest { request, peer_id } => {
                let PendingInboundRequest { request, channel } = request;

                self.task_executor.spawn(
                    handle_request(
                        self.db.clone(),
                        self.seed,
                        self.state_store.clone(),
                        peer_id,
                        request,
                    )
                    .boxed()
                    .compat()
                    .then({
                        let response_channels = self.response_channels.clone();

                        move |result| {
                            match result {
                                Ok(id) => {
                                    let mut response_channels = response_channels.lock().unwrap();
                                    response_channels.insert(id, channel);
                                }
                                Err(response) => channel.send(response).unwrap_or_else(|_| {
                                    log::debug!("failed to send response through channel")
                                }),
                            }
                            Ok(())
                        }
                    }),
                );
            }
        }
    }
}

impl<TSubstream> NetworkBehaviourEventProcess<libp2p::mdns::MdnsEvent> for ComitNode<TSubstream> {
    fn inject_event(&mut self, _event: libp2p::mdns::MdnsEvent) {}
}

fn rfc003_swap_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
    id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    hash_function: HashFunction,
    body: rfc003::messages::RequestBody<AL, BL>,
) -> rfc003::Request<AL, BL, AA, BA> {
    rfc003::Request::<AL, BL, AA, BA> {
        swap_id: id,
        alpha_asset,
        beta_asset,
        alpha_ledger,
        beta_ledger,
        hash_function,
        alpha_ledger_refund_identity: body.alpha_ledger_refund_identity,
        beta_ledger_redeem_identity: body.beta_ledger_redeem_identity,
        alpha_expiry: body.alpha_expiry,
        beta_expiry: body.beta_expiry,
        secret_hash: body.secret_hash,
    }
}

fn build_outbound_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
    request: rfc003::Request<AL, BL, AA, BA>,
) -> Result<frame::OutboundRequest, serde_json::Error> {
    let alpha_ledger_refund_identity = request.alpha_ledger_refund_identity;
    let beta_ledger_redeem_identity = request.beta_ledger_redeem_identity;
    let alpha_expiry = request.alpha_expiry;
    let beta_expiry = request.beta_expiry;
    let secret_hash = request.secret_hash;
    let protocol = SwapProtocol::Rfc003(request.hash_function);

    Ok(frame::OutboundRequest::new("SWAP")
        .with_header("id", request.swap_id.to_header()?)
        .with_header("alpha_ledger", request.alpha_ledger.into().to_header()?)
        .with_header("beta_ledger", request.beta_ledger.into().to_header()?)
        .with_header("alpha_asset", request.alpha_asset.into().to_header()?)
        .with_header("beta_asset", request.beta_asset.into().to_header()?)
        .with_header("protocol", protocol.to_header()?)
        .with_body(serde_json::to_value(rfc003::messages::RequestBody::<
            AL,
            BL,
        > {
            alpha_ledger_refund_identity,
            beta_ledger_redeem_identity,
            alpha_expiry,
            beta_expiry,
            secret_hash,
        })?))
}
