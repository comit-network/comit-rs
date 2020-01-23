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
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Cache<C> {
    pub connector: C,
    #[derivative(Debug = "ignore")]
    pub block_cache: Arc<Mutex<lru::LruCache<Hash, Option<Block>>>>,
    #[derivative(Debug = "ignore")]
    pub receipt_cache: Arc<Mutex<lru::LruCache<Hash, Option<TransactionReceipt>>>>,
}

impl<C> Cache<C> {
    pub fn new(
        connector: C,
        block_cache_capacity: usize,
        receipt_cache_capacity: usize,
    ) -> Cache<C> {
        Cache {
            connector,
            block_cache: Arc::new(Mutex::new(lru::LruCache::new(block_cache_capacity))),
            receipt_cache: Arc::new(Mutex::new(lru::LruCache::new(receipt_cache_capacity))),
        }
    }
}

impl<C> Clone for Cache<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Cache {
            connector: self.connector.clone(),
            block_cache: self.block_cache.clone(),
            receipt_cache: self.receipt_cache.clone(),
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
                if guard.get(&block_hash).is_none() {
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
        let cache = self.block_cache.clone();
        let connector = self.connector.clone();

        let future = async move {
            match cache.lock().await.get(&block_hash) {
                Some(block) => Ok(block.clone()),
                None => {
                    let block = connector.block_by_hash(block_hash.clone()).compat().await?;
                    let mut guard = cache.lock().await;
                    guard.put(block_hash, block.clone());
                    Ok(block)
                }
            }
        }
        .boxed()
        .compat();

        Box::new(future)
    }
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
        let cache = self.receipt_cache.clone();
        let connector = self.connector.clone();

        let future = async move {
            match cache.lock().await.get(&transaction_hash) {
                Some(receipt) => Ok(receipt.clone()),
                None => {
                    let receipt = connector
                        .receipt_by_hash(transaction_hash.clone())
                        .compat()
                        .await?;
                    let mut guard = cache.lock().await;
                    guard.put(transaction_hash, receipt.clone());
                    Ok(receipt)
                }
            }
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}
