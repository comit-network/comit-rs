use crate::swap_protocols::ledger::ethereum::ChainId;
use bitcoin::Network;
use thiserror::Error;

use crate::btsieve::{
    bitcoin::{chain_info, BitcoindConnector},
    ethereum::{net_version, Web3Connector},
};

#[derive(Error, Debug)]
pub enum ConfigValidationError<T: std::fmt::Debug> {
    #[error("Connected network does not match network specified in settings (expected {connected_network:?}, got {specified_network:?})")]
    ConnectedNetworkDoesNotMatchSpecified {
        connected_network: T,
        specified_network: T,
    }
}

#[derive(Debug)]
pub enum NetworkValidationResult<T> {
    Valid,
    Invalid {
        connected_network: T,
        specified_network: T,
    }
}

pub async fn validate_ethereum_chain_id(connection: &Web3Connector, specified: ChainId) -> Result<NetworkValidationResult<ChainId>, anyhow::Error> {
    let actual = net_version(connection).await?;
    if actual == specified {
        Ok(NetworkValidationResult::Valid)
    } else {
        Ok(NetworkValidationResult::Invalid {
            connected_network: actual,
            specified_network: specified,
        })
    }
}

pub async fn validate_bitcoin_network(connection: &BitcoindConnector, specified: Network) -> Result<NetworkValidationResult<Network>, anyhow::Error>  {
    let actual = chain_info(connection).await?;
    if actual.chain == specified {
        Ok(NetworkValidationResult::Valid)
    } else {
        Ok(NetworkValidationResult::Invalid {
            connected_network: actual.chain,
            specified_network: specified,
        })
    }
}
