use async_trait::async_trait;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error<T: Debug> {
    #[error("Connected network does not match network specified in settings (expected {connected_network:?}, got {specified_network:?})")]
    Validation {
        connected_network: T,
        specified_network: T,
    },
}
pub async fn validate_blockchain_config<C, S>(connector: &C, specified: S) -> anyhow::Result<()>
where
    C: FetchNetworkId<S>,
    S: PartialEq + Debug + Send + Sync + 'static,
{
    let actual = connector.network_id().await?;
    if actual == specified {
        Ok(())
    } else {
        Err(anyhow::Error::from(Error::Validation {
            connected_network: actual,
            specified_network: specified,
        }))
    }
}

#[async_trait]
pub trait FetchNetworkId<S>: Send + Sync + 'static {
    async fn network_id(&self) -> anyhow::Result<S>;
}
