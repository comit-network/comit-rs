use crate::btsieve::{BlockByHash, LatestBlock};
use async_trait::async_trait;
use bitcoin::{util::hash::BitcoinHash, Block, BlockHash as Hash, BlockHash};
use derivative::Derivative;
use futures::Future;
use futures_core::{compat::Future01CompatExt, future::TryFutureExt};
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

impl_block_by_hash!();
