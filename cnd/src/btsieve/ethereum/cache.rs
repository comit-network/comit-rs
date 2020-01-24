use crate::{
    btsieve::{
        ethereum::{Block, Hash},
        BlockByHash, LatestBlock, ReceiptByHash,
    },
    ethereum::TransactionReceipt,
};
use derivative::Derivative;
use futures::Future;
use futures_core::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Cache<C> {
    pub connector: C,
    #[derivative(Debug = "ignore")]
    pub block_cache: Arc<Mutex<LruCache<Hash, Option<Block>>>>,
    #[derivative(Debug = "ignore")]
    pub receipt_cache: Arc<Mutex<LruCache<Hash, Option<TransactionReceipt>>>>,
}

impl<C> Cache<C> {
    pub fn new(
        connector: C,
        block_cache_capacity: usize,
        receipt_cache_capacity: usize,
    ) -> Cache<C> {
        Cache {
            connector,
            block_cache: Arc::new(Mutex::new(LruCache::new(block_cache_capacity))),
            receipt_cache: Arc::new(Mutex::new(LruCache::new(receipt_cache_capacity))),
        }
    }
}

impl<C> LatestBlock for Cache<C>
where
    C: LatestBlock<Block = Option<Block>, BlockHash = Hash> + Clone,
{
    type Block = Option<Block>;
    type BlockHash = Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = self.block_cache.clone();
        let mut connector = self.connector.clone();

        let future = async move {
            let block = connector.latest_block().compat().await?;

            if let Some(block) = block.clone() {
                let block_hash = block.hash.expect("no blocks without hash");
                let mut guard = cache.lock().await;
                if !guard.contains(&block_hash) {
                    guard.put(block_hash, Some(block.clone()));
                }
            };

            Ok(block)
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

impl<C> BlockByHash for Cache<C>
where
    C: BlockByHash<Block = Option<Block>, BlockHash = Hash> + Clone,
{
    type Block = Option<Block>;
    type BlockHash = Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let connector = self.connector.clone();
        let cache = Arc::clone(&self.block_cache);
        Box::new(Box::pin(block_by_hash(connector, cache, block_hash)).compat())
    }
}

async fn block_by_hash<C>(
    connector: C,
    cache: Arc<Mutex<LruCache<Hash, Option<Block>>>>,
    block_hash: Hash,
) -> anyhow::Result<Option<Block>>
where
    C: BlockByHash<Block = Option<Block>, BlockHash = Hash> + Clone,
{
    if let Some(block) = cache.lock().await.get(&block_hash) {
        log::trace!("Found block in cache: {:x}", block_hash);
        return Ok(block.clone());
    }

    let block = connector.block_by_hash(block_hash.clone()).compat().await?;
    log::trace!("Fetched block from connector: {:x}", block_hash);

    // We dropped the lock so at this stage the block may have been inserted by
    // another thread, no worries, inserting the same block twice does not hurt.
    let mut guard = cache.lock().await;
    guard.put(block_hash, block.clone());

    Ok(block)
}

impl<C> ReceiptByHash for Cache<C>
where
    C: ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash> + Clone,
{
    type Receipt = Option<TransactionReceipt>;
    type TransactionHash = Hash;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = anyhow::Error> + Send + 'static> {
        let connector = self.connector.clone();
        let cache = Arc::clone(&self.receipt_cache);
        Box::new(Box::pin(receipt_by_hash(connector, cache, transaction_hash)).compat())
    }
}

async fn receipt_by_hash<C>(
    connector: C,
    cache: Arc<Mutex<LruCache<Hash, Option<TransactionReceipt>>>>,
    transaction_hash: Hash,
) -> anyhow::Result<Option<TransactionReceipt>>
where
    C: ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash> + Clone,
{
    if let Some(receipt) = cache.lock().await.get(&transaction_hash) {
        log::trace!("Found receipt in cache: {:x}", transaction_hash);
        return Ok(receipt.clone());
    }

    let receipt = connector
        .receipt_by_hash(transaction_hash.clone())
        .compat()
        .await?;

    log::trace!("Fetched receipt from connector: {:x}", transaction_hash);

    // We dropped the lock so at this stage the receipt may have been inserted by
    // another thread, no worries, inserting the same receipt twice does not hurt.
    let mut guard = cache.lock().await;
    guard.put(transaction_hash, receipt.clone());

    Ok(receipt)
}
