use crate::{
    asset::{self, Asset},
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    db::{AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Sqlite, Swap, SwapTypes},
    network::{
        ComitPeers, DialInformation, ListenAddresses, LocalPeerId, PendingRequestFor, RequestError,
        SendRequest, Swarm,
    },
    seed::{DeriveSwapSeed, RootSeed, SwapSeed},
    swap_protocols::{
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
use chrono::NaiveDateTime;
use futures::sync::oneshot;
use futures_core::future::Either;
use libp2p::{Multiaddr, PeerId};
use libp2p_comit::frame::Response;
use std::sync::Arc;

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug, ambassador::Delegate)]
#[delegate(DeriveSwapSeed, target = "seed")]
#[delegate(LocalPeerId, target = "swarm")]
#[delegate(ComitPeers, target = "swarm")]
#[delegate(ListenAddresses, target = "swarm")]
#[delegate(PendingRequestFor, target = "swarm")]
#[delegate(Retrieve, target = "db")]
#[delegate(DetermineTypes, target = "db")]
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
impl HtlcEvents<Bitcoin, asset::Bitcoin> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Bitcoin>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Bitcoin, asset::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Bitcoin, asset::Bitcoin>,
        htlc_deployment: &Deployed<Bitcoin>,
        htlc_funding: &Funded<Bitcoin, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Either<Redeemed<Bitcoin>, Refunded<Bitcoin>>> {
        self.bitcoin_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding, start_of_swap)
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
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Ethereum>> {
        self.ethereum_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, A>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Ethereum, A>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, A>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, A>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>> {
        self.ethereum_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding, start_of_swap)
            .await
    }
}
