use crate::{
    btsieve::{
        ethereum::{Block, Hash},
        BlockByHash, LatestBlock, ReceiptByHash,
    },
    ethereum::TransactionReceipt,
};
use fmt::Debug;
use futures::Future;
use futures_core::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use std::{fmt, fmt::Formatter, sync::Arc};
use tokio::sync::Mutex;

pub struct EthereumCache<T> {
    pub inner: T,
    pub block_cache: Arc<Mutex<lru::LruCache<Hash, Option<Block>>>>,
    pub receipt_cache: Arc<Mutex<lru::LruCache<Hash, Option<TransactionReceipt>>>>,
}

impl<T> Clone for EthereumCache<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        EthereumCache {
            inner: self.inner.clone(),
            block_cache: self.block_cache.clone(),
            receipt_cache: self.receipt_cache.clone(),
        }
    }
}

impl<T> Debug for EthereumCache<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        self.inner.fmt(f)
    }
}

impl<T> LatestBlock for EthereumCache<T>
where
    T: LatestBlock<Block = Option<Block>, BlockHash = Hash> + Clone,
{
    type Block = Option<Block>;
    type BlockHash = Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = self.block_cache.clone();
        let mut inner = self.inner.clone();

        let future = async move {
            let block = inner.latest_block().compat().await?;

            match block.clone() {
                Some(block) => {
                    let mut guard = cache.lock().await;
                    guard.put(block.hash.expect("no blocks without hash"), Some(block));
                }
                None => (),
            }
            Ok(block)
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

impl<T> BlockByHash for EthereumCache<T>
where
    T: BlockByHash<Block = Option<Block>, BlockHash = Hash> + Clone,
{
    type Block = Option<Block>;
    type BlockHash = Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = self.block_cache.clone();
        let inner = self.inner.clone();

        let future = async move {
            match cache.lock().await.get(&block_hash) {
                Some(block) => Ok(block.clone()),
                None => {
                    let block = inner.block_by_hash(block_hash.clone()).compat().await?;
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

impl<T> ReceiptByHash for EthereumCache<T>
where
    T: ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash> + Clone,
{
    type Receipt = Option<TransactionReceipt>;
    type TransactionHash = Hash;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = anyhow::Error> + Send + 'static> {
        let cache = self.receipt_cache.clone();
        let inner = self.inner.clone();

        let future = async move {
            match cache.lock().await.get(&transaction_hash) {
                Some(receipt) => Ok(receipt.clone()),
                None => {
                    let receipt = inner
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
