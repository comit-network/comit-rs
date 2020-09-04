use crate::{
    btsieve,
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector, LatestBlock},
    ethereum,
};
use comit::btsieve::{ethereum::ReceiptByHash, BlockByHash};
use std::sync::Arc;

/// A facade for accessing various blockchain connectors.
#[derive(Debug, Clone)]
pub struct Connectors {
    bitcoin: Arc<btsieve::bitcoin::Cache<BitcoindConnector>>,
    ethereum: Arc<btsieve::ethereum::Cache<Web3Connector>>,
}

impl Connectors {
    pub fn new(
        bitcoin: btsieve::bitcoin::Cache<BitcoindConnector>,
        ethereum: btsieve::ethereum::Cache<Web3Connector>,
    ) -> Self {
        Self {
            bitcoin: Arc::new(bitcoin),
            ethereum: Arc::new(ethereum),
        }
    }

    /// Provides access to a reference of the Bitcoin connector.
    ///
    /// Most importantly, we don't directly expose the concrete type of the
    /// connector but just which traits it implements.
    pub fn bitcoin(
        &self,
    ) -> Arc<
        impl LatestBlock<Block = bitcoin::Block>
            + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    > {
        self.bitcoin.clone()
    }

    /// Provides access to a reference of the Ethereum connector.
    ///
    /// Most importantly, we don't directly expose the concrete type of the
    /// connector but just which traits it implements.
    pub fn ethereum(
        &self,
    ) -> Arc<
        impl LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
    > {
        self.ethereum.clone()
    }
}
