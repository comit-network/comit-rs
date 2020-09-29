use crate::{
    btsieve::{ethereum::ReceiptByHash, jsonrpc, BlockByHash, LatestBlock},
    ethereum::{ChainId, Hash, TransactionReceipt},
};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug)]
pub struct Web3Connector {
    client: jsonrpc::Client,
}

impl Web3Connector {
    pub fn new(node_url: reqwest::Url) -> Self {
        Self {
            client: jsonrpc::Client::new(node_url),
        }
    }

    pub async fn net_version(&self) -> Result<ChainId> {
        let version = self
            .client
            .send::<Vec<()>, String>(jsonrpc::Request::new("net_version", vec![]))
            .await?;

        Ok(ChainId::from(version.parse::<u32>()?))
    }
}

#[async_trait]
impl LatestBlock for Web3Connector {
    type Block = crate::ethereum::Block;

    async fn latest_block(&self) -> Result<Self::Block> {
        let block: Self::Block = self
            .client
            .send(jsonrpc::Request::new("eth_getBlockByNumber", vec![
                jsonrpc::serialize("latest")?,
                jsonrpc::serialize(true)?,
            ]))
            .await?;

        Ok(block)
    }
}

#[async_trait]
impl BlockByHash for Web3Connector {
    type Block = crate::ethereum::Block;
    type BlockHash = crate::ethereum::Hash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block> {
        let block = self
            .client
            .send(jsonrpc::Request::new("eth_getBlockByHash", vec![
                jsonrpc::serialize(&block_hash)?,
                jsonrpc::serialize(true)?,
            ]))
            .await?;

        Ok(block)
    }
}

#[async_trait]
impl ReceiptByHash for Web3Connector {
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> Result<TransactionReceipt> {
        let receipt = self
            .client
            .send(jsonrpc::Request::new("eth_getTransactionReceipt", vec![
                jsonrpc::serialize(transaction_hash)?,
            ]))
            .await?;

        Ok(receipt)
    }
}
