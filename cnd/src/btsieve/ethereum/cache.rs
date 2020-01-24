use crate::{
    btsieve::{
        ethereum::{self, Hash},
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

// This makes it a bit obscure that we have an option, the compile will point it
// out though; this alias allows us to use the macros :)
type Block = Option<ethereum::Block>;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Cache<C> {
    pub connector: C,
    #[derivative(Debug = "ignore")]
    pub block_cache: Arc<Mutex<LruCache<Hash, Block>>>,
    #[derivative(Debug = "ignore")]
    pub receipt_cache: Arc<Mutex<LruCache<Hash, Option<TransactionReceipt>>>>,
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

impl<C> LatestBlock for Cache<C>
where
    C: LatestBlock<Block = Block, BlockHash = Hash> + Clone,
{
    type Block = Block;
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

impl_block_by_hash!();

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
