use crate::btsieve::{BlockByHash, ConnectedNetwork, LatestBlock};
use anyhow::Result;
use async_trait::async_trait;
use bitcoin::{Block, BlockHash as Hash, BlockHash};
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

    async fn latest_block(&self) -> Result<Self::Block> {
        let block = self.connector.latest_block().await?;

        let block_hash = block.block_hash();
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

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block> {
        if let Some(block) = self.block_cache.lock().await.get(&block_hash) {
            return Ok(block.clone());
        }

        let block = self.connector.block_by_hash(block_hash).await?;

        // We dropped the lock so at this stage the block may have been inserted by
        // another thread, no worries, inserting the same block twice does not hurt.
        let mut guard = self.block_cache.lock().await;
        guard.put(block_hash, block.clone());

        Ok(block)
    }
}

#[async_trait]
impl<N, C> ConnectedNetwork for Cache<C>
where
    C: ConnectedNetwork<Network = N>,
    N: Send + Sync + 'static,
{
    type Network = N;

    async fn connected_network(&self) -> Result<Self::Network> {
        self.connector.connected_network().await
    }
}
