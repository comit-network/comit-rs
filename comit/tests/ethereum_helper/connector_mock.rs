use anyhow::Context;
use async_trait::async_trait;
use comit::{
    btsieve::{ethereum::ReceiptByHash, BlockByHash, ConnectedNetwork, LatestBlock},
    ethereum::{Block, ChainId, Hash, TransactionReceipt},
};
use futures::{stream::BoxStream, StreamExt};
use std::{collections::HashMap, time::Duration};
use tokio::{stream, sync::Mutex, time::throttle};

pub struct EthereumConnectorMock {
    all_blocks: HashMap<Hash, Block>,
    receipts: HashMap<Hash, TransactionReceipt>,
    latest_blocks: Mutex<BoxStream<'static, Block>>,
}

impl EthereumConnectorMock {
    pub fn new(
        latest_blocks: Vec<Block>,
        all_blocks: Vec<Block>,
        receipts: Vec<(Hash, TransactionReceipt)>,
    ) -> Self {
        let all_blocks = all_blocks
            .into_iter()
            .fold(HashMap::new(), |mut hm, block| {
                hm.insert(block.hash, block);
                hm
            });

        EthereumConnectorMock {
            all_blocks,
            receipts: receipts.into_iter().collect(),
            latest_blocks: Mutex::new(
                throttle(Duration::from_secs(1), stream::iter(latest_blocks)).boxed(),
            ),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("there are no more blocks in this blockchain, either your implementation is buggy or you need a better test setup")]
pub struct OutOfBlocks;

#[async_trait]
impl LatestBlock for EthereumConnectorMock {
    type Block = Block;

    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        let block = self
            .latest_blocks
            .lock()
            .await
            .next()
            .await
            .ok_or(OutOfBlocks)?;

        Ok(block)
    }
}

#[async_trait]
impl BlockByHash for EthereumConnectorMock {
    type Block = Block;
    type BlockHash = Hash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        self.all_blocks
            .get(&block_hash)
            .cloned()
            .with_context(|| format!("could not find block with hash {}", block_hash))
    }
}

#[async_trait]
impl ReceiptByHash for EthereumConnectorMock {
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> anyhow::Result<TransactionReceipt> {
        self.receipts
            .get(&transaction_hash)
            .cloned()
            .with_context(|| format!("could not find block with hash {}", transaction_hash))
    }
}

#[async_trait]
impl ConnectedNetwork for EthereumConnectorMock {
    type Network = ChainId;

    async fn connected_network(&self) -> anyhow::Result<Self::Network> {
        Ok(ChainId::GETH_DEV)
    }
}
