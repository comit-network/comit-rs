use anyhow::Context;
use comit::btsieve::ConnectedNetwork;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Connected network does not match network specified in settings (expected {specified_network:?}, got {connected_network:?})")]
pub struct NetworkMismatch<T: Debug> {
    connected_network: T,
    specified_network: T,
}

/// Validate that the connector is connected to the network.
///
/// This function returns a double-result to differentiate between arbitrary
/// connection errors and the network mismatch error.
pub async fn validate_connection_to_network<C, S>(
    connector: &C,
    specified: S,
) -> anyhow::Result<Result<(), NetworkMismatch<S>>>
where
    C: ConnectedNetwork<Network = S>,
    S: PartialEq + Debug + Send + Sync + 'static,
{
    let actual = connector
        .connected_network()
        .await
        .context("failed to determine the connected network")?;

    if actual != specified {
        return Ok(Err(NetworkMismatch {
            connected_network: actual,
            specified_network: specified,
        }));
    }

    Ok(Ok(()))
}
