use crate::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    db::{
        AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Saver, Sqlite, Swap,
        SwapTypes,
    },
    ethereum::{Erc20Token, EtherQuantity},
    network::{DialInformation, Network, RequestError},
    seed::{Seed, SwapSeed},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        rfc003::{
            self,
            events::{HtlcEvents, LedgerEventFutures, LedgerEvents},
            state_machine::SwapStates,
            state_store::{self, InMemoryStateStore, StateStore},
            ActorState, Ledger,
        },
        SwapId,
    },
    CreateLedgerEvents,
};
use async_trait::async_trait;
use bitcoin::Amount;
use futures::{sync::oneshot::Sender, Future};
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
    pub seed: Seed,
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

    fn update<A: ActorState>(&self, key: &SwapId, update: SwapStates<A::AL, A::BL, A::AA, A::BA>) {
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

impl<S> SwapSeed for Facade<S>
where
    S: Send + Sync + 'static,
{
    fn swap_seed(&self, id: SwapId) -> Seed {
        self.seed.swap_seed(id)
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

pub trait LedgerEventsCreator:
    CreateLedgerEvents<Bitcoin, Amount>
    + CreateLedgerEvents<Ethereum, EtherQuantity>
    + CreateLedgerEvents<Ethereum, Erc20Token>
{
}

impl<S> LedgerEventsCreator for Facade<S> where S: Send + Sync + 'static {}

impl<S> CreateLedgerEvents<Bitcoin, Amount> for Facade<S>
where
    S: Send + Sync + 'static,
{
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Bitcoin, Amount>> {
        Box::new(LedgerEventFutures::new(Box::new(
            self.bitcoin_connector.clone(),
        )))
    }
}

impl<S, A> CreateLedgerEvents<Ethereum, A> for Facade<S>
where
    S: Send + Sync + 'static,
    A: Asset + Send + Sync + 'static,
    Web3Connector: HtlcEvents<Ethereum, A>,
{
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, A>> {
        Box::new(LedgerEventFutures::new(Box::new(
            self.ethereum_connector.clone(),
        )))
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
