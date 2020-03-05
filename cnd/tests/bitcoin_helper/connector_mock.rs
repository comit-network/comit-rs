use anyhow::Context;
use async_trait::async_trait;
use bitcoin::{hashes::sha256d, util::hash::BitcoinHash, BlockHash};
use cnd::btsieve::{BlockByHash, LatestBlock};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct BitcoinConnectorMock {
    all_blocks: HashMap<BlockHash, bitcoin::Block>,
    latest_blocks: Vec<bitcoin::Block>,
    latest_time_return_block: Instant,
    current_latest_block_index: usize,
}

impl BitcoinConnectorMock {
    pub fn new(
        latest_blocks: impl IntoIterator<Item = bitcoin::Block>,
        all_blocks: impl IntoIterator<Item = bitcoin::Block>,
    ) -> Self {
        BitcoinConnectorMock {
            all_blocks: all_blocks
                .into_iter()
                .fold(HashMap::new(), |mut hm, block| {
                    hm.insert(block.bitcoin_hash(), block);
                    hm
                }),
            latest_blocks: latest_blocks.into_iter().collect(),
            latest_time_return_block: Instant::now(),
            current_latest_block_index: 0,
        }
    }
}

#[async_trait]
impl LatestBlock for BitcoinConnectorMock {
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    async fn latest_block(&mut self) -> anyhow::Result<Self::Block> {
        if self.latest_blocks.is_empty() {
            return Err(anyhow::Error::from(Error::NoMoreBlocks));
        }

        let latest_block = self.latest_blocks[self.current_latest_block_index].clone();
        if self.latest_time_return_block.elapsed() >= Duration::from_secs(1) {
            self.latest_time_return_block = Instant::now();
            if self
                .latest_blocks
                .get(self.current_latest_block_index + 1)
                .is_some()
            {
                self.current_latest_block_index += 1;
            }
        }
        Ok(latest_block)
    }
}

#[async_trait]
impl BlockByHash for BitcoinConnectorMock {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    async fn block_by_hash(&mut self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
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
