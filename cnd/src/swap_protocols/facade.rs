use crate::{
    asset::{self, Asset},
    btsieve::{self, bitcoin::BitcoindConnector, ethereum, ethereum::Web3Connector},
    db::{AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Sqlite, Swap, SwapTypes},
    network::{
        ComitPeers, DialInformation, ListenAddresses, LocalPeerId, PendingRequestFor, RequestError,
        SendRequest, Swarm,
    },
    seed::{DeriveSwapSeed, RootSeed, SwapSeed},
    swap_protocols::{
        ledger::{bitcoin, Ethereum},
        rfc003::{
            self,
            create_swap::{HtlcParams, SwapEventOnLedger},
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            state_store::{self, InMemoryStateStore, StateStore},
            ActorState, Ledger,
        },
        SwapId,
    },
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use futures_core::channel::oneshot::Sender;
use impl_template::impl_template;
use libp2p::{Multiaddr, PeerId};
use libp2p_comit::frame::{OutboundRequest, Response};
use std::{convert::TryInto, fmt::Debug, sync::Arc};

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
    pub bitcoin_connector: btsieve::bitcoin::Cache<BitcoindConnector>,
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

    fn update<A: ActorState>(
        &self,
        key: &SwapId,
        update: SwapEventOnLedger<<A as ActorState>::AL, <A as ActorState>::BL, A::AA, A::BA>,
    ) {
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
        AL: Ledger,
        BL: Ledger,
        AA: Asset,
        BA: Asset,
        rfc003::Request<AL, BL, AA, BA>: TryInto<OutboundRequest>,
        <rfc003::Request<AL, BL, AA, BA> as TryInto<OutboundRequest>>::Error: Debug,
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

#[impl_template]
#[async_trait::async_trait]
impl HtlcFunded<((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)), asset::Bitcoin>
    for Facade
{
    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<__TYPE0__, asset::Bitcoin, crate::bitcoin::PublicKey>,
        htlc_deployment: &Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<::bitcoin::Transaction, asset::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcDeployed<((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)), asset::Bitcoin>
    for Facade
{
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<__TYPE0__, asset::Bitcoin, crate::bitcoin::PublicKey>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcRedeemed<((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)), asset::Bitcoin>
    for Facade
{
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<__TYPE0__, asset::Bitcoin, crate::bitcoin::PublicKey>,
        htlc_deployment: &Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<::bitcoin::Transaction>> {
        self.bitcoin_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcRefunded<((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)), asset::Bitcoin>
    for Facade
{
    async fn htlc_refunded(
        &self,
        htlc_params: HtlcParams<__TYPE0__, asset::Bitcoin, crate::bitcoin::PublicKey>,
        htlc_deployment: &Deployed<::bitcoin::Transaction, ::bitcoin::OutPoint>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<::bitcoin::Transaction>> {
        self.bitcoin_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcFunded<Ethereum, ((asset::Ether, asset::Erc20))> for Facade {
    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<Ethereum, __TYPE0__, crate::ethereum::Address>,
        htlc_deployment: &Deployed<crate::ethereum::Transaction, crate::ethereum::Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<crate::ethereum::Transaction, __TYPE0__>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcDeployed<Ethereum, ((asset::Ether, asset::Erc20))> for Facade {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<Ethereum, __TYPE0__, crate::ethereum::Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<crate::ethereum::Transaction, crate::ethereum::Address>> {
        self.ethereum_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcRedeemed<Ethereum, ((asset::Ether, asset::Erc20))> for Facade {
    async fn htlc_redeemed(
        &self,
        htlc_params: HtlcParams<Ethereum, __TYPE0__, crate::ethereum::Address>,
        htlc_deployment: &Deployed<crate::ethereum::Transaction, crate::ethereum::Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<crate::ethereum::Transaction>> {
        self.ethereum_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl HtlcRefunded<Ethereum, ((asset::Ether, asset::Erc20))> for Facade {
    async fn htlc_refunded(
        &self,
        htlc_params: HtlcParams<Ethereum, __TYPE0__, crate::ethereum::Address>,
        htlc_deployment: &Deployed<crate::ethereum::Transaction, crate::ethereum::Address>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<crate::ethereum::Transaction>> {
        self.ethereum_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}
