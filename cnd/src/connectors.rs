use crate::{
    btsieve,
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector, LatestBlock},
    ethereum,
    http_api::LedgerNotConfigured,
};
use anyhow::Result;
use comit::{
    btsieve::{ethereum::ReceiptByHash, BlockByHash},
    lnd::{LndConnectorAsReceiver, LndConnectorAsSender, LndConnectorParams},
};
use std::sync::Arc;

/// A facade for accessing various blockchain connectors.
#[derive(Debug, Clone)]
pub struct Connectors {
    bitcoin: Arc<btsieve::bitcoin::Cache<BitcoindConnector>>,
    ethereum: Arc<btsieve::ethereum::Cache<Web3Connector>>,
    lnd_connector_params: Option<LndConnectorParams>,
}

impl Connectors {
    pub fn new(
        bitcoin: btsieve::bitcoin::Cache<BitcoindConnector>,
        ethereum: btsieve::ethereum::Cache<Web3Connector>,
        lnd_connector_params: Option<LndConnectorParams>,
    ) -> Self {
        Self {
            bitcoin: Arc::new(bitcoin),
            ethereum: Arc::new(ethereum),
            lnd_connector_params,
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

    pub fn lnd_as_sender(&self) -> Result<LndConnectorAsSender> {
        let params = self.lnd_connector_params()?;

        Ok(LndConnectorAsSender::from(params))
    }

    pub fn lnd_as_receiver(&self) -> Result<LndConnectorAsReceiver> {
        let params = self.lnd_connector_params()?;

        Ok(LndConnectorAsReceiver::from(params))
    }

    pub fn supports_halbit(&self) -> Result<()> {
        self.lnd_connector_params().map(|_| ())
    }

    fn lnd_connector_params(&self) -> Result<LndConnectorParams> {
        let params = self
            .lnd_connector_params
            .clone()
            .ok_or_else(|| LedgerNotConfigured {
                ledger: "lightning",
            })?;

        Ok(params)
    }
}
