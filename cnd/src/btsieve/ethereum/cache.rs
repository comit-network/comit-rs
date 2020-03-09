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
    C: LatestBlock<Block = Block> + Clone,
{
    type Block = Block;

    async fn latest_block(&mut self) -> anyhow::Result<Self::Block> {
        let cache = self.block_cache.clone();
        let mut connector = self.connector.clone();

        let block = connector.latest_block().await?;

        let block_hash = block.hash.expect("no blocks without hash");
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
    type BlockHash = Hash;

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

#[async_trait]
impl<C> ReceiptByHash for Cache<C>
where
    C: ReceiptByHash + Clone,
{
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> anyhow::Result<TransactionReceipt> {
        let connector = self.connector.clone();
        let cache = Arc::clone(&self.receipt_cache);

        if let Some(receipt) = cache.lock().await.get(&transaction_hash) {
            tracing::trace!("Found receipt in cache: {:x}", transaction_hash);
            return Ok(receipt.clone());
        }

        let receipt = connector.receipt_by_hash(transaction_hash.clone()).await?;

        tracing::trace!("Fetched receipt from connector: {:x}", transaction_hash);

        // We dropped the lock so at this stage the receipt may have been inserted by
        // another thread, no worries, inserting the same receipt twice does not hurt.
        let mut guard = cache.lock().await;
        guard.put(transaction_hash, receipt.clone());

        Ok(receipt)
    }
}
