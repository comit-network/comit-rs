pub mod comit;
#[cfg(test)]
pub mod test_swarm;
pub mod transport;

pub use ::comit::*;
pub use transport::ComitTransport;

use crate::{
    asset::{AssetKind, Erc20},
    btsieve::{
        bitcoin::{self, BitcoindConnector},
        ethereum::{self, Web3Connector},
    },
    comit_api::LedgerKind,
    config::Settings,
    db::{CreatedSwap, ForSwap, Rfc003Swap, Save, Sqlite},
    htlc_location,
    http_api::LedgerNotConfigured,
    libp2p_comit_ext::{FromHeader, ToHeader},
    network::comit::Comit,
    seed::RootSeed,
    swap_protocols::{
        halight, hbit, herc20,
        rfc003::{
            self,
            messages::{Decision, DeclineResponseBody, Request, RequestBody, SwapDeclineReason},
            state::Insert,
            LedgerState, SwapCommunication, SwapCommunicationStates, SwapId,
        },
        state::Update,
        HashFunction, Herc20HalightBitcoinCreateSwapParams, LocalSwapId, Role, SwapProtocol,
    },
    transaction,
};
use ::comit::lnd::{LndConnectorAsReceiver, LndConnectorAsSender, LndConnectorParams};
use anyhow::Context;
use async_trait::async_trait;
use chrono::Utc;
use futures::{
    channel::oneshot::{self, Sender},
    stream::StreamExt,
    Future, FutureExt, TryStreamExt,
};
use libp2p::{
    identity::{ed25519, Keypair},
    mdns::Mdns,
    swarm::SwarmBuilder,
    Multiaddr, NetworkBehaviour, PeerId,
};
use libp2p_comit::{
    frame::{OutboundRequest, ValidatedInboundRequest},
    BehaviourOutEvent, PendingInboundRequest, Rfc003Comit,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    convert::{TryFrom, TryInto},
    fmt::Debug,
    io,
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
};
use tokio::{runtime::Handle, sync::Mutex};
use tracing_futures::Instrument;

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
#[allow(clippy::type_complexity)]
pub struct Swarm {
    #[derivative(Debug = "ignore")]
    inner: Arc<Mutex<libp2p::Swarm<ComitNode>>>,
    local_peer_id: PeerId,
}

impl Swarm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        settings: &Settings,
        seed: RootSeed,
        bitcoin_connector: Arc<bitcoin::Cache<BitcoindConnector>>,
        ethereum_connector: Arc<ethereum::Cache<Web3Connector>>,
        lnd_connector_params: Option<LndConnectorParams>,
        swap_communication_states: Arc<SwapCommunicationStates>,
        rfc003_alpha_ledger_states: Arc<rfc003::LedgerStates>,
        rfc003_beta_ledger_states: Arc<rfc003::LedgerStates>,
        herc20_states: Arc<herc20::States>,
        halight_states: Arc<halight::States>,
        database: &Sqlite,
        task_executor: tokio::runtime::Handle,
    ) -> anyhow::Result<Self> {
        let local_key_pair = derive_key_pair(&seed);
        let local_peer_id = PeerId::from(local_key_pair.clone().public());
        tracing::info!("Starting with peer_id: {}", local_peer_id);

        let transport = transport::build_comit_transport(local_key_pair)?;
        let behaviour = ComitNode::new(
            bitcoin_connector,
            ethereum_connector,
            lnd_connector_params,
            swap_communication_states,
            rfc003_alpha_ledger_states,
            rfc003_beta_ledger_states,
            herc20_states,
            halight_states,
            seed,
            database.clone(),
            task_executor.clone(),
        )?;

        let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id.clone())
            .executor(Box::new(TokioExecutor {
                handle: task_executor,
            }))
            .build();

        for addr in settings.network.listen.clone() {
            libp2p::Swarm::listen_on(&mut swarm, addr.clone())
                .with_context(|| format!("Address is not supported: {:?}", addr))?;
        }

        let swarm = Arc::new(Mutex::new(swarm));

        Ok(Self {
            inner: swarm,
            local_peer_id,
        })
    }

    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;

        guard.initiate_communication(id, swap_params)
    }

    pub async fn get_created_swap(
        &self,
        id: LocalSwapId,
    ) -> Option<Herc20HalightBitcoinCreateSwapParams> {
        let mut guard = self.inner.lock().await;
        guard.get_created_swap(id)
    }
}

struct TokioExecutor {
    handle: tokio::runtime::Handle,
}

impl libp2p::core::Executor for TokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let _ = self.handle.spawn(future);
    }
}

/// The SwarmWorker, when spawned into a runtime, continuously polls the
/// underlying swarm for events.
///
/// This is the main driver of the networking code.
/// Note that the inner swarm is wrapped in an `Arc<Mutex>` and we only hold the
/// lock for a short period of time, giving other parts of the code also the
/// opportunity to acquire the lock and interact with the network.
#[derive(Debug)]
pub struct SwarmWorker {
    pub swarm: Swarm,
}

impl futures::Future for SwarmWorker {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let mutex = self.swarm.inner.lock();
            futures::pin_mut!(mutex);

            let mut guard = futures::ready!(mutex.poll(cx));
            futures::ready!(guard.poll_next_unpin(cx));
        }
    }
}

fn derive_key_pair(seed: &RootSeed) -> Keypair {
    let bytes = seed.sha256_with_seed(&[b"NODE_ID"]);
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
    Keypair::Ed25519(key.into())
}

/// A `NetworkBehaviour` that delegates to the `Comit` and `Mdns` behaviours.
#[derive(NetworkBehaviour)]
#[allow(missing_debug_implementations)]
pub struct ComitNode {
    rfc003_comit: Rfc003Comit,
    comit: Comit,
    /// Multicast DNS discovery network behaviour.
    mdns: Mdns,

    // blockchain connectors
    #[behaviour(ignore)]
    pub bitcoin_connector: Arc<bitcoin::Cache<BitcoindConnector>>,
    #[behaviour(ignore)]
    pub ethereum_connector: Arc<ethereum::Cache<Web3Connector>>,
    #[behaviour(ignore)]
    lnd_connector_params: Option<LndConnectorParams>,

    #[behaviour(ignore)]
    pub seed: RootSeed,
    #[behaviour(ignore)]
    pub db: Sqlite,
    #[behaviour(ignore)]
    task_executor: Handle,

    // rfc003
    #[behaviour(ignore)]
    pub swap_communication_states: Arc<SwapCommunicationStates>,
    #[behaviour(ignore)]
    pub rfc003_alpha_ledger_states: Arc<rfc003::LedgerStates>,
    #[behaviour(ignore)]
    pub rfc003_beta_ledger_states: Arc<rfc003::LedgerStates>,
    #[behaviour(ignore)]
    response_channels: Arc<Mutex<HashMap<SwapId, oneshot::Sender<libp2p_comit::frame::Response>>>>,

    #[behaviour(ignore)]
    pub herc20_states: Arc<herc20::States>,
    #[behaviour(ignore)]
    pub halight_states: Arc<halight::States>,
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

impl ComitNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bitcoin_connector: Arc<bitcoin::Cache<BitcoindConnector>>,
        ethereum_connector: Arc<ethereum::Cache<Web3Connector>>,
        lnd_connector_params: Option<LndConnectorParams>,
        swap_communication_states: Arc<SwapCommunicationStates>,
        rfc003_alpha_ledger_states: Arc<rfc003::LedgerStates>,
        rfc003_beta_ledger_states: Arc<rfc003::LedgerStates>,
        herc20_states: Arc<herc20::States>,
        halight_states: Arc<halight::States>,
        seed: RootSeed,
        db: Sqlite,
        task_executor: Handle,
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
            rfc003_comit: Rfc003Comit::new(known_headers),
            mdns: Mdns::new()?,
            comit: Comit::new(seed),
            bitcoin_connector,
            ethereum_connector,
            rfc003_alpha_ledger_states,
            rfc003_beta_ledger_states,
            herc20_states,
            halight_states,
            swap_communication_states,
            seed,
            db,
            response_channels: Arc::new(Mutex::new(HashMap::new())),
            task_executor,
            lnd_connector_params,
        })
    }

    pub fn send_request(
        &mut self,
        peer_id: DialInformation,
        request: OutboundRequest,
    ) -> impl futures::Future<Output = Result<libp2p_comit::frame::Response, ()>> + Send + 'static + Unpin
    {
        self.rfc003_comit
            .send_request((peer_id.peer_id, peer_id.address_hint), request)
    }

    pub fn initiate_communication(
        &mut self,
        id: LocalSwapId,
        swap_params: Herc20HalightBitcoinCreateSwapParams,
    ) -> anyhow::Result<()> {
        self.supports_halight()?;
        self.comit.initiate_communication(id, swap_params)
    }

    fn init_hbit_herc20(
        &mut self,
        id: LocalSwapId,
        swap: CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>,
    ) -> anyhow::Result<()> {
        self.comit.init_hbit_herc20(id, swap)
    }

    fn init_herc20_hbit(
        &mut self,
        id: LocalSwapId,
        swap: CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>,
    ) -> anyhow::Result<()> {
        self.comit.init_herc20_hbit(id, swap)
    }

    pub fn get_created_swap(
        &mut self,
        id: LocalSwapId,
    ) -> Option<Herc20HalightBitcoinCreateSwapParams> {
        self.comit.get_created_swap(&id)
    }

    fn supports_halight(&self) -> anyhow::Result<()> {
        match self.lnd_connector_params {
            Some(_) => Ok(()),
            None => Err(anyhow::Error::from(LedgerNotConfigured {
                ledger: "lightning",
            })),
        }
    }
}

/// Init the communication protocols.
#[async_trait]
pub trait InitCommunication<T> {
    async fn init_communication(&self, swap_id: LocalSwapId, created_swap: T)
        -> anyhow::Result<()>;
}

#[async_trait]
impl InitCommunication<CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>> for Swarm {
    async fn init_communication(
        &self,
        swap_id: LocalSwapId,
        created_swap: CreatedSwap<hbit::CreatedSwap, herc20::CreatedSwap>,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        guard.init_hbit_herc20(swap_id, created_swap)
    }
}

#[async_trait]
impl InitCommunication<CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>> for Swarm {
    async fn init_communication(
        &self,
        swap_id: LocalSwapId,
        created_swap: CreatedSwap<herc20::CreatedSwap, hbit::CreatedSwap>,
    ) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        guard.init_herc20_hbit(swap_id, created_swap)
    }
}

async fn handle_request(
    db: Sqlite,
    swap_communication_states: Arc<SwapCommunicationStates>,
    alpha_ledger_states: Arc<rfc003::LedgerStates>,
    beta_ledger_states: Arc<rfc003::LedgerStates>,
    counterparty: PeerId,
    mut request: ValidatedInboundRequest,
) -> Result<SwapId, libp2p_comit::frame::Response> {
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
                            insert_state_for_bob::<
                                _,
                                _,
                                _,
                                _,
                                htlc_location::Bitcoin,
                                htlc_location::Ethereum,
                                _,
                                _,
                                transaction::Bitcoin,
                                transaction::Ethereum,
                                _,
                            >(
                                db.clone(),
                                swap_communication_states.clone(),
                                alpha_ledger_states.clone(),
                                beta_ledger_states.clone(),
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
                            insert_state_for_bob::<
                                _,
                                _,
                                _,
                                _,
                                htlc_location::Ethereum,
                                htlc_location::Bitcoin,
                                _,
                                _,
                                transaction::Ethereum,
                                transaction::Bitcoin,
                                _,
                            >(
                                db.clone(),
                                swap_communication_states.clone(),
                                alpha_ledger_states.clone(),
                                beta_ledger_states.clone(),
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
                            insert_state_for_bob::<
                                _,
                                _,
                                _,
                                _,
                                htlc_location::Bitcoin,
                                htlc_location::Ethereum,
                                _,
                                _,
                                transaction::Bitcoin,
                                transaction::Ethereum,
                                _,
                            >(
                                db.clone(),
                                swap_communication_states.clone(),
                                alpha_ledger_states.clone(),
                                beta_ledger_states.clone(),
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
                            insert_state_for_bob::<
                                _,
                                _,
                                _,
                                _,
                                htlc_location::Ethereum,
                                htlc_location::Bitcoin,
                                _,
                                _,
                                transaction::Ethereum,
                                transaction::Bitcoin,
                                _,
                            >(
                                db.clone(),
                                swap_communication_states.clone(),
                                alpha_ledger_states.clone(),
                                beta_ledger_states.clone(),
                                counterparty,
                                request,
                            )
                            .await
                            .expect("Could not save state to db");
                            Ok(swap_id)
                        }
                        (alpha_ledger, beta_ledger, alpha_asset, beta_asset) => {
                            tracing::warn!(
                                    "swapping {:?} to {:?} from {:?} to {:?} is currently not supported", alpha_asset, beta_asset, alpha_ledger, beta_ledger
                                );

                            let decline_body = DeclineResponseBody {
                                reason: Some(SwapDeclineReason::UnsupportedSwap),
                            };

                            Err(libp2p_comit::frame::Response::empty()
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
            tracing::warn!("request type '{}' is unknown", request_type);

            Err(libp2p_comit::frame::Response::empty().with_header(
                "decision",
                Decision::Declined
                    .to_header()
                    .expect("Decision should not fail to serialize"),
            ))
        }
    }
}

#[allow(clippy::type_complexity)]
async fn insert_state_for_bob<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT, DB>(
    db: DB,
    swap_communication_states: Arc<SwapCommunicationStates>,
    alpha_ledger_state: Arc<rfc003::LedgerStates>,
    beta_ledger_state: Arc<rfc003::LedgerStates>,
    counterparty: PeerId,
    swap_request: Request<AL, BL, AA, BA, AI, BI>,
) -> anyhow::Result<()>
where
    AL: Send + 'static,
    BL: Send + 'static,
    AA: Ord + Send + 'static,
    BA: Ord + Send + 'static,
    AH: Send + 'static,
    BH: Send + 'static,
    AI: Send + 'static,
    BI: Send + 'static,
    AT: Send + 'static,
    BT: Send + 'static,
    DB: Save<Request<AL, BL, AA, BA, AI, BI>> + Save<Rfc003Swap>,
    Request<AL, BL, AA, BA, AI, BI>: Clone,
{
    let id = swap_request.swap_id;

    Save::save(&db, Rfc003Swap::new(id, Role::Bob, counterparty)).await?;
    Save::save(&db, swap_request.clone()).await?;

    swap_communication_states
        .insert(id, SwapCommunication::Proposed {
            request: swap_request,
        })
        .await;

    alpha_ledger_state
        .insert(id, LedgerState::<AA, AH, AT>::NotDeployed)
        .await;
    beta_ledger_state
        .insert(id, LedgerState::<BA, BH, BT>::NotDeployed)
        .await;

    Ok(())
}

/// Get the `PeerId` of this node.
#[ambassador::delegatable_trait]
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
#[ambassador::delegatable_trait]
#[allow(clippy::type_complexity)]
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
        let mut swarm = self.inner.lock().await;
        Box::new(swarm.rfc003_comit.connected_peers())
    }
}

/// IP addresses local node is listening on.
#[async_trait]
#[ambassador::delegatable_trait]
pub trait ListenAddresses {
    async fn listen_addresses(&self) -> Vec<Multiaddr>;
}

#[async_trait]
impl ListenAddresses for Swarm {
    async fn listen_addresses(&self) -> Vec<Multiaddr> {
        let swarm = self.inner.lock().await;

        libp2p::Swarm::listeners(&swarm)
            .chain(libp2p::Swarm::external_addresses(&swarm))
            .cloned()
            .collect()
    }
}

/// Get pending network requests for swap.
#[async_trait]
#[ambassador::delegatable_trait]
pub trait PendingRequestFor {
    async fn pending_request_for(
        &self,
        swap: SwapId,
    ) -> Option<Sender<libp2p_comit::frame::Response>>;
}

#[async_trait]
impl PendingRequestFor for Swarm {
    async fn pending_request_for(
        &self,
        swap: SwapId,
    ) -> Option<Sender<libp2p_comit::frame::Response>> {
        let swarm = self.inner.lock().await;
        let mut response_channels = swarm.response_channels.lock().await;
        response_channels.remove(&swap)
    }
}

/// Send swap request to connected peer.
#[async_trait]
pub trait SendRequest {
    async fn send_request<AL, BL, AA, BA, AI, BI>(
        &self,
        peer_identity: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA, AI, BI>,
    ) -> Result<rfc003::Response<AI, BI>, RequestError>
    where
        rfc003::messages::AcceptResponseBody<AI, BI>: DeserializeOwned,
        rfc003::Request<AL, BL, AA, BA, AI, BI>: TryInto<OutboundRequest> + Send + 'static + Clone,
        <rfc003::Request<AL, BL, AA, BA, AI, BI> as TryInto<OutboundRequest>>::Error: Debug;
}

#[async_trait]
impl SendRequest for Swarm {
    async fn send_request<AL, BL, AA, BA, AI, BI>(
        &self,
        dial_information: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA, AI, BI>,
    ) -> Result<rfc003::Response<AI, BI>, RequestError>
    where
        rfc003::messages::AcceptResponseBody<AI, BI>: DeserializeOwned,
        rfc003::Request<AL, BL, AA, BA, AI, BI>: TryInto<OutboundRequest> + Send + 'static + Clone,
        <rfc003::Request<AL, BL, AA, BA, AI, BI> as TryInto<OutboundRequest>>::Error: Debug,
    {
        let id = request.swap_id;
        let request = request
            .try_into()
            .expect("constructing a frame::OutgoingRequest should never fail!");

        let result = {
            let mut guard = self.inner.lock().await;
            let swarm = &mut *guard;

            tracing::debug!(
                "Making swap request to {}: {:?}",
                dial_information.clone(),
                id,
            );

            swarm.send_request(dial_information.clone(), request)
        }
        .await;

        match result {
            Ok(mut response) => {
                let decision = response
                    .take_header("decision")
                    .map(Decision::from_header)
                    .map_or(Ok(None), |x| x.map(Some))
                    .map_err(|e| {
                        tracing::error!(
                            "Could not deserialize header in response {:?}: {}",
                            response,
                            e,
                        );
                        RequestError::InvalidResponse
                    })?;

                match decision {
                    Some(Decision::Accepted) => {
                        let accept_body =
                            rfc003::messages::AcceptResponseBody::deserialize(response.body());

                        match accept_body {
                            Ok(body) => Ok(Ok(rfc003::Accept {
                                swap_id: id,
                                beta_ledger_refund_identity: body.beta_ledger_refund_identity,
                                alpha_ledger_redeem_identity: body.alpha_ledger_redeem_identity,
                            })),
                            Err(_e) => Err(RequestError::InvalidResponse),
                        }
                    }

                    Some(Decision::Declined) => {
                        let decline_body =
                            rfc003::messages::DeclineResponseBody::deserialize(response.body());

                        match decline_body {
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
                tracing::error!(
                    "Unable to request over connection {:?}:{:?}",
                    dial_information,
                    e
                );
                Err(RequestError::Connection)
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<BehaviourOutEvent> for ComitNode {
    fn inject_event(&mut self, event: BehaviourOutEvent) {
        match event {
            BehaviourOutEvent::PendingInboundRequest { request, peer_id } => {
                let PendingInboundRequest { request, channel } = request;

                let response_channels = self.response_channels.clone();
                let db = self.db.clone();
                let swap_communication_states = self.swap_communication_states.clone();
                let alpha_ledger_state = self.rfc003_alpha_ledger_states.clone();
                let beta_ledger_state = self.rfc003_beta_ledger_states.clone();

                self.task_executor.spawn(async move {
                    match handle_request(
                        db,
                        swap_communication_states,
                        alpha_ledger_state,
                        beta_ledger_state,
                        peer_id,
                        request,
                    )
                    .await
                    {
                        Ok(id) => {
                            let mut response_channels = response_channels.lock().await;
                            response_channels.insert(id, channel);
                        }
                        Err(response) => channel.send(response).unwrap_or_else(|_| {
                            tracing::debug!("failed to send response through channel")
                        }),
                    }
                });
            }
        }
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<libp2p::mdns::MdnsEvent> for ComitNode {
    fn inject_event(&mut self, _event: libp2p::mdns::MdnsEvent) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ComitNode {
    fn inject_event(&mut self, _event: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<comit::BehaviourOutEvent> for ComitNode {
    fn inject_event(&mut self, event: comit::BehaviourOutEvent) {
        match event {
            comit::BehaviourOutEvent::SwapFinalized {
                local_swap_id,
                swap_params: create_swap_params,
                secret_hash,
                ethereum_identity,
                lightning_identity,
            } => {
                let role = create_swap_params.role;

                // first, we save what we learned to the db
                let db = self.db.clone();
                let save_to_db_task = match role {
                    Role::Alice => async move {
                        db.save(ForSwap {
                            local_swap_id,
                            data: network::WhatAliceLearnedFromBob {
                                redeem_ethereum_identity: ethereum_identity,
                                refund_lightning_identity: lightning_identity,
                            },
                        })
                        .await
                        .context("failed to save what alice learned from bob")
                    }
                    .boxed(),
                    Role::Bob => async move {
                        db.save(ForSwap {
                            local_swap_id,
                            data: network::WhatBobLearnedFromAlice {
                                secret_hash,
                                refund_ethereum_identity: ethereum_identity,
                                redeem_lightning_identity: lightning_identity,
                            },
                        })
                        .await
                        .context("failed to save what alice learned from bob")
                    }
                    .boxed(),
                };
                self.task_executor.spawn(async move {
                    let result = save_to_db_task.await;
                    if let Err(e) = result {
                        tracing::error!("{:?}", e);
                    }
                });

                // second, we spawn the watcher for halight
                let lnd_connector_params = match self.lnd_connector_params.clone() {
                    Some(lnd_connector_params) => lnd_connector_params,
                    None => {
                        tracing::error!("Internal Failure: lnd connectors are not initialised, no action has been taken. This should be unreachable.");
                        return;
                    }
                };

                let (halight_redeem_identity, halight_refund_identity) = match role {
                    Role::Alice => (create_swap_params.lightning_identity, lightning_identity),
                    Role::Bob => (lightning_identity, create_swap_params.lightning_identity),
                };

                let halight_params = halight::Params {
                    redeem_identity: halight_redeem_identity,
                    refund_identity: halight_refund_identity,
                    cltv_expiry: create_swap_params.lightning_cltv_expiry,
                    asset: create_swap_params.lightning_amount,
                    secret_hash,
                };

                let halight_states = self.halight_states.clone();
                let halight_watcher_task = match role {
                    Role::Alice => async move {
                        let lnd_connector = LndConnectorAsReceiver::from(lnd_connector_params);

                        halight::new_halight_swap(
                            local_swap_id,
                            halight_params,
                            halight_states,
                            lnd_connector,
                        )
                        .await;
                    }
                    .boxed(),
                    Role::Bob => async move {
                        let lnd_connector = LndConnectorAsSender::from(lnd_connector_params);

                        halight::new_halight_swap(
                            local_swap_id,
                            halight_params,
                            halight_states,
                            lnd_connector,
                        )
                        .await;
                    }
                    .boxed(),
                };
                self.task_executor.spawn(halight_watcher_task.instrument(
                    tracing::error_span!("beta_ledger", swap_id = %local_swap_id, role = %role),
                ));

                // third, we spawn the watcher for herc20
                let (herc20_redeem_identity, herc20_refund_identity) = match role {
                    Role::Alice => (ethereum_identity, create_swap_params.ethereum_identity),
                    Role::Bob => (create_swap_params.ethereum_identity, ethereum_identity),
                };
                let params = herc20::Params {
                    asset: Erc20::new(
                        create_swap_params.token_contract,
                        create_swap_params.ethereum_amount,
                    ),
                    redeem_identity: herc20_redeem_identity,
                    refund_identity: herc20_refund_identity,
                    expiry: create_swap_params.ethereum_absolute_expiry,
                    secret_hash,
                };
                let start_of_swap = Utc::now().naive_local();
                let connector = self.ethereum_connector.clone();
                let states = self.herc20_states.clone();

                self.task_executor.spawn(async move {
                    let mut events = herc20::new(connector.as_ref(), params, start_of_swap)
                        .instrument(
                            tracing::error_span!("alpha_ledger", swap_id = %local_swap_id, role = %role),
                        )
                        .inspect_ok(|event| tracing::info!("yielded event {}", event))
                        .inspect_err(|error| tracing::error!("swap failed with {:?}", error));

                    while let Ok(Some(event)) = events.try_next().await {
                        states.update(&local_swap_id, event).await;
                    }

                    tracing::info!("swap finished");
                });
            }
        }
    }
}

impl<AL, BL, AA, BA, AI, BI> TryFrom<Request<AL, BL, AA, BA, AI, BI>> for OutboundRequest
where
    RequestBody<AI, BI>: From<Request<AL, BL, AA, BA, AI, BI>> + Serialize,
    LedgerKind: From<AL> + From<BL>,
    AssetKind: From<AA> + From<BA>,
    Request<AL, BL, AA, BA, AI, BI>: Clone,
{
    type Error = anyhow::Error;

    fn try_from(request: Request<AL, BL, AA, BA, AI, BI>) -> anyhow::Result<Self> {
        let request_body = RequestBody::from(request.clone());
        let protocol = SwapProtocol::Rfc003(request.hash_function).to_header()?;

        let alpha_ledger = LedgerKind::from(request.alpha_ledger).to_header()?;
        let beta_ledger = LedgerKind::from(request.beta_ledger).to_header()?;
        let alpha_asset = AssetKind::from(request.alpha_asset).to_header()?;
        let beta_asset = AssetKind::from(request.beta_asset).to_header()?;

        Ok(OutboundRequest::new("SWAP")
            .with_header("id", request.swap_id.to_header()?)
            .with_header("alpha_ledger", alpha_ledger)
            .with_header("beta_ledger", beta_ledger)
            .with_header("alpha_asset", alpha_asset)
            .with_header("beta_asset", beta_asset)
            .with_header("protocol", protocol)
            .with_body(serde_json::to_value(request_body)?))
    }
}

impl<AL, BL, AA, BA, AI, BI> From<Request<AL, BL, AA, BA, AI, BI>> for RequestBody<AI, BI> {
    fn from(request: Request<AL, BL, AA, BA, AI, BI>) -> Self {
        RequestBody {
            alpha_ledger_refund_identity: request.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: request.beta_ledger_redeem_identity,
            alpha_expiry: request.alpha_expiry,
            beta_expiry: request.beta_expiry,
            secret_hash: request.secret_hash,
        }
    }
}

fn rfc003_swap_request<AL, BL, AA, BA, AI, BI>(
    id: SwapId,
    alpha_ledger: AL,
    beta_ledger: BL,
    alpha_asset: AA,
    beta_asset: BA,
    hash_function: HashFunction,
    body: rfc003::messages::RequestBody<AI, BI>,
) -> rfc003::Request<AL, BL, AA, BA, AI, BI> {
    rfc003::Request {
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
