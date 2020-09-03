use crate::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    ethereum::ChainId,
};
use anyhow::Context;
use async_trait::async_trait;
use comit::ledger;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Connected network does not match network specified in settings (expected {specified_network:?}, got {connected_network:?})")]
pub struct NetworkMismatch<T: Debug> {
    connected_network: T,
    specified_network: T,
}

#[derive(Error, Debug, Copy, Clone)]
#[error("connection failure")]
pub struct ConnectionFailure;

/// Validate that the connector is connected to the network.
///
/// This function returns a double-result to differentiate between arbitrary
/// connection errors and the network mismatch error.
pub async fn validate_connection_to_network<C, S>(
    connector: &C,
    specified: S,
) -> anyhow::Result<Result<(), NetworkMismatch<S>>>
where
    C: FetchNetworkId<S>,
    S: PartialEq + Debug + Send + Sync + 'static,
{
    let actual = connector.network_id().await.context(ConnectionFailure)?;

    if actual != specified {
        return Ok(Err(NetworkMismatch {
            connected_network: actual,
            specified_network: specified,
        }));
    }

    Ok(Ok(()))
}

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
