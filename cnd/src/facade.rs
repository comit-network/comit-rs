use crate::{
    connectors::Connectors,
    ethereum,
    network::{Identities, SwapDigest, Swarm},
    storage::{Load, LoadAll, Save, Storage},
    LocalSwapId, Role, Timestamp,
};
use comit::bitcoin;
use libp2p::{Multiaddr, PeerId};

/// This is a facade that implements all the required traits and forwards them
/// to another implementation. This allows us to keep the number of arguments to
/// HTTP API controllers small and still access all the functionality we need.
#[derive(Clone, Debug)]
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
        let timestamp = ethereum::latest_time(self.connectors.ethereum.as_ref()).await?;

        Ok(timestamp)
    }

    pub async fn dial_addr(&mut self, addr: Multiaddr) -> anyhow::Result<()> {
        self.swarm.dial_addr(addr).await
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
