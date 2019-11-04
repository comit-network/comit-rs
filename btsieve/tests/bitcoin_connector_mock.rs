use bitcoin::{hashes::sha256d, util::hash::BitcoinHash};
use btsieve::{BlockByHash, LatestBlock};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::prelude::{Future, IntoFuture};

#[derive(Clone)]
pub struct BitcoinConnectorMock {
    all_blocks: HashMap<sha256d::Hash, bitcoin::Block>,
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

impl LatestBlock for BitcoinConnectorMock {
    type Error = ();
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        if self.latest_blocks.is_empty() {
            return Box::new(Err(()).into_future());
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
        Box::new(Ok(latest_block).into_future())
    }
}

impl BlockByHash for BitcoinConnectorMock {
    type Error = ();
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(
            self.all_blocks
                .get(&block_hash)
                .cloned()
                .ok_or(())
                .into_future(),
        )
    }
}
