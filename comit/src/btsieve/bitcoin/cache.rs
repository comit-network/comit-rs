use crate::{
    btsieve::{BlockByHash, LatestBlock},
    Timestamp,
};
use async_trait::async_trait;
use bitcoin::{util::hash::BitcoinHash, Block, BlockHash as Hash, BlockHash};
use derivative::Derivative;
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Cache<C> {
    pub connector: C,
    #[derivative(Debug = "ignore")]
    pub block_cache: Arc<Mutex<LruCache<BlockHash, Block>>>,
}

impl<C> Cache<C> {
    pub fn new(connector: C, capacity: usize) -> Cache<C> {
        let block_cache = Arc::new(Mutex::new(LruCache::new(capacity)));
        Cache {
            connector,
            block_cache,
        }
    }
}

#[async_trait]
impl<C> LatestBlock for Cache<C>
where
    C: LatestBlock<Block = Block>,
{
    type Block = Block;

    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        let block = self.connector.latest_block().await?;

        let block_hash = block.bitcoin_hash();
        let mut guard = self.block_cache.lock().await;
        if !guard.contains(&block_hash) {
            guard.put(block_hash, block.clone());
        }

        Ok(block)
    }
}

#[async_trait]
impl<C> BlockByHash for Cache<C>
where
    C: BlockByHash<Block = Block, BlockHash = Hash>,
{
    type Block = Block;
    type BlockHash = BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        if let Some(block) = self.block_cache.lock().await.get(&block_hash) {
            tracing::trace!("Found block in cache: {:x}", block_hash);
            return Ok(block.clone());
        }

        let block = self.connector.block_by_hash(block_hash.clone()).await?;
        tracing::trace!("Fetched block from connector: {:x}", block_hash);

        // We dropped the lock so at this stage the block may have been inserted by
        // another thread, no worries, inserting the same block twice does not hurt.
        let mut guard = self.block_cache.lock().await;
        guard.put(block_hash, block.clone());

        Ok(block)
    }
}

impl<C> Cache<C>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash>,
{
    /// Median time past is defined as the median of the blocktimes from the
    /// last 11 blocks.
    pub async fn median_time_past(&self) -> anyhow::Result<Timestamp> {
        let mut block_times = vec![];

        let mut current = self.latest_block().await?;
        block_times.push(current.header.time);

        for _ in 0..10 {
            let prev = current.header.prev_blockhash;
            current = self.block_by_hash(prev).await?;
            block_times.push(current.header.time);
        }

        block_times.sort();
        let median = block_times[5];
        Ok(Timestamp::from(median))
    }
}
