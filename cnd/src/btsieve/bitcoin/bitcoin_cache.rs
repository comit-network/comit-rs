use crate::btsieve::{BlockByHash, LatestBlock};
use bitcoin::{hashes::sha256d, util::hash::BitcoinHash};
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
pub struct BitcoinCache<T> {
    pub inner: T,
    #[derivative(Debug = "ignore")]
    pub cache: Arc<Mutex<lru::LruCache<sha256d::Hash, bitcoin::Block>>>,
}

impl<T> Clone for BitcoinCache<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        BitcoinCache {
            inner: self.inner.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl<T> LatestBlock for BitcoinCache<T>
where
    T: LatestBlock<Block = bitcoin::Block, BlockHash = sha256d::Hash> + Clone,
{
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = self.cache.clone();
        let mut inner = self.inner.clone();

        let future = async move {
            let block = inner.latest_block().compat().await?;
            let mut guard = cache.lock().await;
            guard.put(block.bitcoin_hash(), block.clone());
            Ok(block)
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

impl<T> BlockByHash for BitcoinCache<T>
where
    T: BlockByHash<Block = bitcoin::Block, BlockHash = sha256d::Hash> + Clone,
{
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = self.cache.clone();
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
