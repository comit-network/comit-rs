use crate::{
    btsieve::LatestBlock,
    connectors::Connectors,
    network::{ComitPeers, DialInformation, Identities, Swarm},
    storage::{Load, LoadAll, Storage},
    LocalSwapId, Role, Save, Timestamp,
};
use ::comit::network::protocols::announce::SwapDigest;
use comit::bitcoin;
use libp2p::{Multiaddr, PeerId};

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug, ambassador::Delegate)]
#[delegate(ComitPeers, target = "swarm")]
pub struct Facade {
    pub swarm: Swarm,
    pub storage: Storage,
    pub connectors: Connectors,
}

impl Facade {
    pub async fn initiate_communication(
        &self,
        id: LocalSwapId,
        peer: DialInformation,
        role: Role,
        digest: SwapDigest,
        identities: Identities,
    ) -> anyhow::Result<()> {
        self.swarm
            .initiate_communication(id, peer, role, digest, identities)
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
