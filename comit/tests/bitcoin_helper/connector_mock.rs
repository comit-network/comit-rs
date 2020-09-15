use anyhow::Context;
use async_trait::async_trait;
use bitcoin::BlockHash;
use comit::btsieve::{BlockByHash, LatestBlock};
use futures::{stream::BoxStream, StreamExt};
use std::{collections::HashMap, time::Duration};
use tokio::{stream, sync::Mutex, time::throttle};

pub struct BitcoinConnectorMock {
    all_blocks: HashMap<BlockHash, bitcoin::Block>,
    latest_blocks: Mutex<BoxStream<'static, bitcoin::Block>>,
}

impl BitcoinConnectorMock {
    pub fn new(latest_blocks: Vec<bitcoin::Block>, all_blocks: Vec<bitcoin::Block>) -> Self {
        BitcoinConnectorMock {
            all_blocks: all_blocks
                .into_iter()
                .fold(HashMap::new(), |mut hm, block| {
                    hm.insert(block.block_hash(), block);
                    hm
                }),
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
impl LatestBlock for BitcoinConnectorMock {
    type Block = bitcoin::Block;

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
impl BlockByHash for BitcoinConnectorMock {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        self.all_blocks
            .get(&block_hash)
            .cloned()
            .with_context(|| format!("could not find block with hash {}", block_hash))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ran out of blocks in chain")]
    NoMoreBlocks,
}
