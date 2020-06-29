use crate::{
    btsieve::{
        ethereum::{self, Hash, ReceiptByHash},
        BlockByHash, LatestBlock,
    },
    ethereum::TransactionReceipt,
};
use async_trait::async_trait;
use derivative::Derivative;
use lru::LruCache;
pub use primitive_types::U256;
use std::sync::Arc;
use tokio::sync::Mutex;

// This makes it a bit obscure that we have an option, the compile will point it
// out though; this alias allows us to use the macros :)
type Block = ethereum::Block;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Cache<C> {
    pub connector: C,
    #[derivative(Debug = "ignore")]
    pub block_cache: Arc<Mutex<LruCache<Hash, Block>>>,
    #[derivative(Debug = "ignore")]
    pub receipt_cache: Arc<Mutex<LruCache<Hash, TransactionReceipt>>>,
}

impl<C> Cache<C> {
    pub fn new(
        connector: C,
        block_cache_capacity: usize,
        receipt_cache_capacity: usize,
    ) -> Cache<C> {
        let block_cache = Arc::new(Mutex::new(LruCache::new(block_cache_capacity)));
        let receipt_cache = Arc::new(Mutex::new(LruCache::new(receipt_cache_capacity)));
        Cache {
            connector,
            block_cache,
            receipt_cache,
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

        let mut guard = self.block_cache.lock().await;
        if !guard.contains(&block.hash) {
            guard.put(block.hash, block.clone());
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
    type BlockHash = Hash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        if let Some(block) = self.block_cache.lock().await.get(&block_hash) {
            tracing::trace!("Found block in cache: {}", block_hash);
            return Ok(block.clone());
        }

        let block = self.connector.block_by_hash(block_hash).await?;
        tracing::trace!("Fetched block from connector: {}", block_hash);

        // We dropped the lock so at this stage the block may have been inserted by
        // another thread, no worries, inserting the same block twice does not hurt.
        let mut guard = self.block_cache.lock().await;
        guard.put(block_hash, block.clone());

        Ok(block)
    }
}

#[async_trait]
impl<C> ReceiptByHash for Cache<C>
where
    C: ReceiptByHash,
{
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> anyhow::Result<TransactionReceipt> {
        if let Some(receipt) = self.receipt_cache.lock().await.get(&transaction_hash) {
            tracing::trace!("Found receipt in cache: {}", transaction_hash);
            return Ok(receipt.clone());
        }

        let receipt = self.connector.receipt_by_hash(transaction_hash).await?;

        tracing::trace!("Fetched receipt from connector: {}", transaction_hash);

        // We dropped the lock so at this stage the receipt may have been inserted by
        // another thread, no worries, inserting the same receipt twice does not hurt.
        let mut guard = self.receipt_cache.lock().await;
        guard.put(transaction_hash, receipt.clone());

        Ok(receipt)
    }
}
