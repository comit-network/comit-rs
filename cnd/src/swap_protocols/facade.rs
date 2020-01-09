use crate::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    db::{
        AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Saver, Sqlite, Swap,
        SwapTypes,
    },
    network::{DialInformation, Network, RequestError},
    seed::{DeriveSwapSeed, RootSeed, SwapSeed},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            create_swap::{HtlcParams, SwapEvent},
            events::{Deployed, Funded, HtlcEvents, Redeemed, Refunded},
            state_store::{self, InMemoryStateStore, StateStore},
            ActorState, Ledger,
        },
        SwapId,
    },
};
use async_trait::async_trait;
use bitcoin::Amount;
use futures::{sync::oneshot::Sender, Future};
use futures_core::future::Either;
use libp2p::PeerId;
use libp2p_comit::frame::Response;
use std::sync::Arc;
use tokio::{executor, runtime::TaskExecutor};

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[allow(missing_debug_implementations)]
pub struct Facade<S> {
    pub bitcoin_connector: BitcoindConnector,
    pub ethereum_connector: Web3Connector,
    pub state_store: Arc<InMemoryStateStore>,
    pub seed: RootSeed,
    pub swarm: Arc<S>, // S is the libp2p Swarm within a mutex.
    pub db: Sqlite,
    pub task_executor: TaskExecutor,
}

impl<S> Clone for Facade<S> {
    fn clone(&self) -> Self {
        Self {
            bitcoin_connector: self.bitcoin_connector.clone(),
            ethereum_connector: self.ethereum_connector.clone(),
            state_store: Arc::clone(&self.state_store),
            seed: self.seed,
            swarm: Arc::clone(&self.swarm),
            db: self.db.clone(),
            task_executor: self.task_executor.clone(),
        }
    }
}

impl<S> StateStore for Facade<S>
where
    S: Send + Sync + 'static,
{
    fn insert<A: ActorState>(&self, key: SwapId, value: A) {
        self.state_store.insert(key, value)
    }

    fn get<A: ActorState>(&self, key: &SwapId) -> Result<Option<A>, state_store::Error> {
        self.state_store.get(key)
    }

    fn update<A: ActorState>(&self, key: &SwapId, update: SwapEvent<A::AL, A::BL, A::AA, A::BA>) {
        self.state_store.update::<A>(key, update)
    }
}

impl<S: Network> Network for Facade<S>
where
    S: Send + Sync + 'static,
{
    fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<libp2p::Multiaddr>)> + Send + 'static> {
        self.swarm.comit_peers()
    }

    fn listen_addresses(&self) -> Vec<libp2p::Multiaddr> {
        self.swarm.listen_addresses()
    }

    fn pending_request_for(&self, swap: SwapId) -> Option<Sender<Response>> {
        self.swarm.pending_request_for(swap)
    }

    fn send_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
        &self,
        peer_identity: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Box<dyn Future<Item = rfc003::Response<AL, BL>, Error = RequestError> + Send + 'static>
    {
        self.swarm.send_request(peer_identity, request)
    }
}

impl<S> DeriveSwapSeed for Facade<S>
where
    S: Send + Sync + 'static,
{
    fn derive_swap_seed(&self, id: SwapId) -> SwapSeed {
        self.seed.derive_swap_seed(id)
    }
}

#[async_trait]
impl<S> Retrieve for Facade<S>
where
    S: Send + Sync + 'static,
{
    async fn get(&self, key: &SwapId) -> anyhow::Result<Swap> {
        self.db.get(key).await
    }

    async fn all(&self) -> anyhow::Result<Vec<Swap>> {
        self.db.all().await
    }
}

#[async_trait]
impl<S, AL, BL, AA, BA> LoadAcceptedSwap<AL, BL, AA, BA> for Facade<S>
where
    S: Send + Sync + 'static,
    AL: Ledger + Send + 'static,
    BL: Ledger + Send + 'static,
    AA: Asset + Send + 'static,
    BA: Asset + Send + 'static,
    Sqlite: LoadAcceptedSwap<AL, BL, AA, BA>,
{
    async fn load_accepted_swap(
        &self,
        swap_id: &SwapId,
    ) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA>> {
        self.db.load_accepted_swap(swap_id).await
    }
}

#[async_trait]
impl<S> DetermineTypes for Facade<S>
where
    S: Send + Sync + 'static,
{
    async fn determine_types(&self, key: &SwapId) -> anyhow::Result<SwapTypes> {
        self.db.determine_types(key).await
    }
}

#[async_trait]
impl<S> Saver for Facade<S> where S: Send + Sync + 'static {}

#[async_trait]
impl<S, T> Save<T> for Facade<S>
where
    S: Send + Sync + 'static,
    T: Send + 'static,
    Sqlite: Save<T>,
{
    async fn save(&self, data: T) -> anyhow::Result<()> {
        self.db.save(data).await
    }
}

#[async_trait::async_trait]
impl<S> HtlcEvents<Bitcoin, Amount> for Facade<S>
where
    S: Send + Sync + 'static,
{
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
    ) -> Result<Deployed<Bitcoin>, rfc003::Error> {
        self.bitcoin_connector.htlc_deployed(htlc_params).await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
        htlc_deployment: &Deployed<Bitcoin>,
    ) -> Result<Funded<Bitcoin, Amount>, rfc003::Error> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Bitcoin, Amount>,
        htlc_deployment: &Deployed<Bitcoin>,
        htlc_funding: &Funded<Bitcoin, Amount>,
    ) -> Result<Either<Redeemed<Bitcoin>, Refunded<Bitcoin>>, rfc003::Error> {
        self.bitcoin_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding)
            .await
    }
}

#[async_trait::async_trait]
impl<A, S> HtlcEvents<Ethereum, A> for Facade<S>
where
    S: Send + Sync + 'static,
    A: Asset + Send + Sync + 'static,
    Web3Connector: HtlcEvents<Ethereum, A>,
{
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, A>,
    ) -> Result<Deployed<Ethereum>, rfc003::Error> {
        self.ethereum_connector.htlc_deployed(htlc_params).await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, A>,
        htlc_deployment: &Deployed<Ethereum>,
    ) -> Result<Funded<Ethereum, A>, rfc003::Error> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, A>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, A>,
    ) -> Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>, rfc003::Error> {
        self.ethereum_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding)
            .await
    }
}

impl<S> executor::Executor for Facade<S>
where
    S: Send + Sync + 'static,
{
    fn spawn(
        &mut self,
        future: Box<dyn Future<Item = (), Error = ()> + Send>,
    ) -> Result<(), executor::SpawnError> {
        executor::Executor::spawn(&mut self.task_executor, future)
    }
}
