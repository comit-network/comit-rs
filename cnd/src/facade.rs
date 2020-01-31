use crate::{
    comit_api::LedgerKind,
    db::{DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Sqlite, SwapTypes},
    network::{
        ComitPeers, DialInformation, ListenAddresses, LocalPeerId, PendingRequestFor, RequestError,
        SendRequest, Swarm,
    },
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use comit::{
    asset::{self, Asset},
    btsieve::{bitcoin, bitcoin::BitcoindConnector, ethereum, ethereum::Web3Connector},
    seed::{DeriveSwapSeed, RootSeed, SwapSeed},
    swap_protocols::{
        ledger::{self, Ethereum},
        rfc003::{
            self,
            create_swap::{HtlcParams, SwapEvent},
            events::{Deployed, Funded, HtlcEvents, Redeemed, Refunded},
            state_store::{self, InMemoryStateStore, StateStore},
            AcceptedSwap, ActorState, Ledger, Swap,
        },
        SwapId,
    },
};
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
    pub bitcoin_connector: bitcoin::Cache<BitcoindConnector>,
    pub ethereum_connector: ethereum::Cache<Web3Connector>,
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
    async fn send_request<AL, BL, AA, BA>(
        &self,
        peer_identity: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA>,
    ) -> Result<rfc003::Response<AL, BL>, RequestError>
    where
        AL: rfc003::Ledger + Into<LedgerKind>,
        BL: rfc003::Ledger + Into<LedgerKind>,
        AA: Asset,
        BA: Asset,
    {
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
impl<B: ledger::bitcoin::Bitcoin + 'static> HtlcEvents<B, asset::Bitcoin> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<B, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<B>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<B, asset::Bitcoin>,
        htlc_deployment: &Deployed<B>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<B, asset::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<B, asset::Bitcoin>,
        htlc_deployment: &Deployed<B>,
        htlc_funding: &Funded<B, asset::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Either<Redeemed<B>, Refunded<B>>> {
        self.bitcoin_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcEvents<Ethereum, asset::Ether> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Ethereum>> {
        self.ethereum_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Ethereum, asset::Ether>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Ether>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, asset::Ether>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>> {
        self.ethereum_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl HtlcEvents<Ethereum, asset::Erc20> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Erc20>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<Ethereum>> {
        self.ethereum_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }

    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Erc20>,
        htlc_deployment: &Deployed<Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<Ethereum, asset::Erc20>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }

    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, asset::Erc20>,
        htlc_deployment: &Deployed<Ethereum>,
        htlc_funding: &Funded<Ethereum, asset::Erc20>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Either<Redeemed<Ethereum>, Refunded<Ethereum>>> {
        self.ethereum_connector
            .htlc_redeemed_or_refunded(htlc_params, htlc_deployment, htlc_funding, start_of_swap)
            .await
    }
}
