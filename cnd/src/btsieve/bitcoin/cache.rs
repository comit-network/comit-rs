use crate::btsieve::{BlockByHash, LatestBlock};
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
    C: LatestBlock<Block = Block, BlockHash = BlockHash> + Clone,
{
    type Block = Block;
    type BlockHash = BlockHash;

    async fn latest_block(&mut self) -> anyhow::Result<Self::Block> {
        let cache = Arc::clone(&self.block_cache);
        let mut connector = self.connector.clone();

        let block = connector.latest_block().await?;

        let block_hash = block.bitcoin_hash();
        let mut guard = cache.lock().await;
        if !guard.contains(&block_hash) {
            guard.put(block_hash, block.clone());
        }

        Ok(block)
    }
}

#[async_trait]
impl<C> BlockByHash for Cache<C>
where
    C: BlockByHash<Block = Block, BlockHash = Hash> + Clone,
{
    type Block = Block;
    type BlockHash = BlockHash;

    async fn block_by_hash(&mut self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        let mut connector = self.connector.clone();
        let cache = Arc::clone(&self.block_cache);

        if let Some(block) = cache.lock().await.get(&block_hash) {
            tracing::trace!("Found block in cache: {:x}", block_hash);
            return Ok(block.clone());
        }

        let block = connector.block_by_hash(block_hash.clone()).await?;
        tracing::trace!("Fetched block from connector: {:x}", block_hash);

        // We dropped the lock so at this stage the block may have been inserted by
        // another thread, no worries, inserting the same block twice does not hurt.
        let mut guard = cache.lock().await;
        guard.put(block_hash, block.clone());

        Ok(block)
    }
}
