use crate::btsieve::{
    bitcoin::{self, BitcoindConnector},
    ethereum::{self, Web3Connector},
};
use std::sync::Arc;

/// Blockchain connectors.
#[derive(Debug, Clone)]
pub struct Connectors {
    pub bitcoin: Arc<bitcoin::Cache<BitcoindConnector>>,
    pub ethereum: Arc<ethereum::Cache<Web3Connector>>,
}
