use crate::{
    btsieve::LatestBlock,
    connectors::Connectors,
    network::{ComitPeers, Identities, ListenAddresses, LocalPeerId, NewOrder, SwapDigest, Swarm},
    storage::{Load, LoadAll, Save, Storage},
    LocalSwapId, Role, Timestamp,
};
use comit::{
    bitcoin, identity,
    network::{Order, OrderId},
};
use libp2p::{Multiaddr, PeerId};

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug, ambassador::Delegate)]
#[delegate(ComitPeers, target = "swarm")]
#[delegate(ListenAddresses, target = "swarm")]
#[delegate(LocalPeerId, target = "swarm")]
pub struct Facade {
    pub swarm: Swarm,
    pub storage: Storage,
    pub connectors: Connectors,
}

impl Facade {
    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
        peer: PeerId,
        address_hint: Option<Multiaddr>,
    ) -> anyhow::Result<()> {
        self.swarm
            .initiate_communication(id, role, digest, identities, peer, address_hint)
            .await
    }

    /// Returns the current Bitcoin median time past.
    pub async fn bitcoin_median_time_past(&self) -> anyhow::Result<Timestamp> {
        let timestamp = bitcoin::median_time_past(self.connectors.bitcoin.as_ref()).await?;

        Ok(timestamp)
    }

    /// Returns the timestamp of the latest Ethereum block.
    pub async fn ethereum_latest_time(&self) -> anyhow::Result<Timestamp> {
        let timestamp = self
            .connectors
            .ethereum
            .latest_block()
            .await?
            .timestamp
            .into();

        Ok(timestamp)
    }

    pub async fn take_order(
        &mut self,
        order_id: OrderId,
        swap_id: LocalSwapId,
        bitcoin_identity: crate::bitcoin::Address,
        ethereum_identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        self.storage
            .associate_swap_with_order(order_id, swap_id)
            .await;

        self.swarm
            .take_order(order_id, swap_id, bitcoin_identity, ethereum_identity)
            .await
    }

    pub async fn make_order(
        &self,
        order: NewOrder,
        swap_id: LocalSwapId,
        ethereum_identity: identity::Ethereum,
        bitcoin_identity: crate::bitcoin::Address,
    ) -> anyhow::Result<OrderId> {
        self.swarm
            .make_order(order, swap_id, ethereum_identity, bitcoin_identity)
            .await
    }

    pub async fn get_order(&self, order_id: OrderId) -> Option<Order> {
        self.swarm.get_order(order_id).await
    }

    pub async fn get_orders(&self) -> Vec<Order> {
        self.swarm.get_orders().await
    }

    pub async fn dial_addr(&mut self, addr: Multiaddr) {
        let _ = self.swarm.dial_addr(addr).await;
    }
}

#[async_trait::async_trait]
impl<T> Save<T> for Facade
where
    Storage: Save<T>,
    T: Send + 'static,
{
    async fn save(&self, data: T) -> anyhow::Result<()> {
        self.storage.save(data).await
    }
}

#[async_trait::async_trait]
impl<T> Load<T> for Facade
where
    Storage: Load<T>,
    T: Send + 'static,
{
    async fn load(&self, swap_id: LocalSwapId) -> anyhow::Result<T> {
        self.storage.load(swap_id).await
    }
}

#[async_trait::async_trait]
impl<T> LoadAll<T> for Facade
where
    Storage: LoadAll<T>,
    T: Send + 'static,
{
    async fn load_all(&self) -> anyhow::Result<Vec<T>> {
        self.storage.load_all().await
    }
}
