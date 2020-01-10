use crate::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    db::{
        AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Saver, Sqlite, Swap,
        SwapTypes,
    },
    network::{
        ComitPeers, DialInformation, ListenAddresses, LocalPeerId, PendingRequestFor, RequestError,
        SendRequest, Swarm,
    },
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
use futures::sync::oneshot::Sender;
use futures_core::future::Either;
use libp2p::{Multiaddr, PeerId};
use libp2p_comit::frame::Response;
use std::sync::Arc;

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug)]
pub struct Facade {
    pub bitcoin_connector: BitcoindConnector,
    pub ethereum_connector: Web3Connector,
    pub state_store: Arc<InMemoryStateStore>,
    pub seed: RootSeed,
    pub swarm: Swarm,
    pub db: Sqlite,
}

impl StateStore for Facade {
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

impl LocalPeerId for Facade {
    fn local_peer_id(&self) -> PeerId {
        self.swarm.local_peer_id()
    }
}

#[async_trait]
impl ComitPeers for Facade {
    async fn comit_peers(
        &self,
    ) -> Box<dyn Iterator<Item = (PeerId, Vec<Multiaddr>)> + Send + 'static> {
        self.swarm.comit_peers().await
    }
}

#[async_trait]
impl ListenAddresses for Facade {
    async fn listen_addresses(&self) -> Vec<Multiaddr> {
        self.swarm.listen_addresses().await
    }
}

#[async_trait]
impl PendingRequestFor for Facade {
    async fn pending_request_for(&self, swap: SwapId) -> Option<Sender<Response>> {
        self.swarm.pending_request_for(swap).await
    }
}

#[async_trait]
impl SendRequest for Facade {
    async fn send_request<AL: rfc003::Ledger, BL: rfc003::Ledger, AA: Asset, BA: Asset>(
        &self,
        peer_identity: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<rfc003::Response<AL, BL>, RequestError> {
        self.swarm.send_request(peer_identity, request).await
    }
}

impl DeriveSwapSeed for Facade {
    fn derive_swap_seed(&self, id: SwapId) -> SwapSeed {
        self.seed.derive_swap_seed(id)
    }
}

#[async_trait]
impl Retrieve for Facade {
    async fn get(&self, key: &SwapId) -> anyhow::Result<Swap> {
        self.db.get(key).await
    }

    async fn all(&self) -> anyhow::Result<Vec<Swap>> {
        self.db.all().await
    }
}

#[async_trait]
impl<AL, BL, AA, BA> LoadAcceptedSwap<AL, BL, AA, BA> for Facade
where
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
impl DetermineTypes for Facade {
    async fn determine_types(&self, key: &SwapId) -> anyhow::Result<SwapTypes> {
        self.db.determine_types(key).await
    }
}

#[async_trait]
impl Saver for Facade {}

#[async_trait]
impl<T> Save<T> for Facade
where
    T: Send + 'static,
    Sqlite: Save<T>,
{
    async fn save(&self, data: T) -> anyhow::Result<()> {
        self.db.save(data).await
    }
}

#[async_trait::async_trait]
impl HtlcEvents<Bitcoin, Amount> for Facade {
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
impl<A> HtlcEvents<Ethereum, A> for Facade
where
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
