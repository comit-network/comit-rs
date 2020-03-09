use crate::{
    asset,
    btsieve::{
        self,
        bitcoin::BitcoindConnector,
        ethereum::{self, Web3Connector},
    },
    db::{AcceptedSwap, DetermineTypes, LoadAcceptedSwap, Retrieve, Save, Sqlite, Swap, SwapTypes},
    htlc_location, identity,
    network::{
        ComitPeers, DialInformation, ListenAddresses, LocalPeerId, PendingRequestFor, RequestError,
        SendRequest, Swarm,
    },
    seed::{DeriveSwapSeed, RootSeed, SwapSeed},
    swap_protocols::{
        ledger::{bitcoin, Ethereum},
        rfc003::{
            self,
            create_swap::{HtlcParams, SwapEvent},
            events::{
                Deployed, Funded, HtlcDeployed, HtlcFunded, HtlcRedeemed, HtlcRefunded, Redeemed,
                Refunded,
            },
            ActorState,
        },
        state_store::{self, InMemoryStateStore, StateStore},
        SwapId,
    },
    transaction,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use futures::channel::oneshot::Sender;
use impl_template::impl_template;
use libp2p::{Multiaddr, PeerId};
use libp2p_comit::frame::{OutboundRequest, Response};
use serde::de::DeserializeOwned;
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
    pub bitcoin_connector: Arc<btsieve::bitcoin::Cache<BitcoindConnector>>,
    pub ethereum_connector: Arc<ethereum::Cache<Web3Connector>>,
    pub state_store: Arc<InMemoryStateStore>,
    pub seed: RootSeed,
    pub swarm: Swarm,
    pub db: Sqlite,
}

impl<S> StateStore<S, SwapEvent<S::AA, S::BA, S::AH, S::BH, S::AT, S::BT>> for Facade
where
    S: ActorState + Clone + Send,
    S::AA: Ord,
    S::BA: Ord,
{
    fn insert(&self, key: SwapId, value: S) {
        StateStore::<S, SwapEvent<S::AA, S::BA, S::AH, S::BH, S::AT, S::BT>>::insert(
            self.state_store.as_ref(),
            key,
            value,
        )
    }

    fn get(&self, key: &SwapId) -> Result<Option<S>, state_store::Error> {
        StateStore::<S, SwapEvent<S::AA, S::BA, S::AH, S::BH, S::AT, S::BT>>::get(
            self.state_store.as_ref(),
            key,
        )
    }

    #[allow(clippy::type_complexity)]
    fn update(&self, key: &SwapId, update: SwapEvent<S::AA, S::BA, S::AH, S::BH, S::AT, S::BT>) {
        StateStore::<S, SwapEvent<S::AA, S::BA, S::AH, S::BH, S::AT, S::BT>>::update(
            self.state_store.as_ref(),
            key,
            update,
        )
    }
}

#[async_trait]
impl SendRequest for Facade {
    async fn send_request<AL, BL, AA, BA, AI, BI>(
        &self,
        peer_identity: DialInformation,
        request: rfc003::Request<AL, BL, AA, BA, AI, BI>,
    ) -> Result<rfc003::Response<AI, BI>, RequestError>
    where
        rfc003::messages::AcceptResponseBody<AI, BI>: DeserializeOwned,
        rfc003::Request<AL, BL, AA, BA, AI, BI>: TryInto<OutboundRequest> + Send + 'static + Clone,
        <rfc003::Request<AL, BL, AA, BA, AI, BI> as TryInto<OutboundRequest>>::Error: Debug,
    {
        self.swarm.send_request(peer_identity, request).await
    }
}

#[async_trait]
impl<AL, BL, AA, BA, AI, BI> LoadAcceptedSwap<AL, BL, AA, BA, AI, BI> for Facade
where
    Sqlite: LoadAcceptedSwap<AL, BL, AA, BA, AI, BI>,
    AcceptedSwap<AL, BL, AA, BA, AI, BI>: Send + 'static,
{
    async fn load_accepted_swap(
        &self,
        swap_id: &SwapId,
    ) -> anyhow::Result<AcceptedSwap<AL, BL, AA, BA, AI, BI>> {
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
impl
    HtlcFunded<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Bitcoin,
        htlc_location::Bitcoin,
        identity::Bitcoin,
        transaction::Bitcoin,
    > for Facade
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<__TYPE0__, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<asset::Bitcoin, transaction::Bitcoin>> {
        self.bitcoin_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcDeployed<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Bitcoin,
        htlc_location::Bitcoin,
        identity::Bitcoin,
        transaction::Bitcoin,
    > for Facade
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<__TYPE0__, asset::Bitcoin, identity::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<htlc_location::Bitcoin, transaction::Bitcoin>> {
        self.bitcoin_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcRedeemed<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Bitcoin,
        htlc_location::Bitcoin,
        identity::Bitcoin,
        transaction::Bitcoin,
    > for Facade
{
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams<__TYPE0__, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<transaction::Bitcoin>> {
        self.bitcoin_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcRefunded<
        ((bitcoin::Mainnet, bitcoin::Testnet, bitcoin::Regtest)),
        asset::Bitcoin,
        htlc_location::Bitcoin,
        identity::Bitcoin,
        transaction::Bitcoin,
    > for Facade
{
    async fn htlc_refunded(
        &self,
        htlc_params: &HtlcParams<__TYPE0__, asset::Bitcoin, identity::Bitcoin>,
        htlc_deployment: &Deployed<htlc_location::Bitcoin, transaction::Bitcoin>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<transaction::Bitcoin>> {
        self.bitcoin_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcFunded<
        Ethereum,
        ((asset::Ether, asset::Erc20)),
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Facade
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<Ethereum, __TYPE0__, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<__TYPE0__, transaction::Ethereum>> {
        self.ethereum_connector
            .htlc_funded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcDeployed<
        Ethereum,
        ((asset::Ether, asset::Erc20)),
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Facade
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<Ethereum, __TYPE0__, identity::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<htlc_location::Ethereum, transaction::Ethereum>> {
        self.ethereum_connector
            .htlc_deployed(htlc_params, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcRedeemed<
        Ethereum,
        ((asset::Ether, asset::Erc20)),
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Facade
{
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams<Ethereum, __TYPE0__, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<transaction::Ethereum>> {
        self.ethereum_connector
            .htlc_redeemed(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}

#[impl_template]
#[async_trait::async_trait]
impl
    HtlcRefunded<
        Ethereum,
        ((asset::Ether, asset::Erc20)),
        htlc_location::Ethereum,
        identity::Ethereum,
        transaction::Ethereum,
    > for Facade
{
    async fn htlc_refunded(
        &self,
        htlc_params: &HtlcParams<Ethereum, __TYPE0__, identity::Ethereum>,
        htlc_deployment: &Deployed<htlc_location::Ethereum, transaction::Ethereum>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<transaction::Ethereum>> {
        self.ethereum_connector
            .htlc_refunded(htlc_params, htlc_deployment, start_of_swap)
            .await
    }
}
