use async_trait::async_trait;
use comit::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    ethereum::ChainId,
    ledger,
};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Connected network does not match network specified in settings (expected {connected_network:?}, got {specified_network:?})")]
pub struct NetworkMismatch<T: Debug> {
    connected_network: T,
    specified_network: T,
}

#[derive(Error, Debug, Copy, Clone)]
#[error("connection failure")]
pub struct ConnectionFailure;

#[async_trait]
pub trait FetchNetworkId<S>: Send + Sync + 'static {
    async fn network_id(&self) -> anyhow::Result<S>;
}

#[async_trait]
impl FetchNetworkId<ledger::Bitcoin> for BitcoindConnector {
    async fn network_id(&self) -> anyhow::Result<ledger::Bitcoin> {
        let chain = self.chain_info().await?.chain;

        Ok(chain)
    }
}

#[async_trait]
impl FetchNetworkId<ChainId> for Web3Connector {
    async fn network_id(&self) -> anyhow::Result<ChainId> {
        let chain_id = self.net_version().await?;

        Ok(chain_id)
    }
}
