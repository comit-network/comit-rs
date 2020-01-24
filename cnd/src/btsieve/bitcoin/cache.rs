use crate::btsieve::{BlockByHash, LatestBlock};
use bitcoin::{
    hashes::{sha256d, sha256d::Hash},
    util::hash::BitcoinHash,
    Block,
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
    pub cache: Arc<Mutex<LruCache<sha256d::Hash, bitcoin::Block>>>,
}

impl<T> Cache<T> {
    pub fn new(connector: T, capacity: usize) -> Cache<T> {
        Cache {
            connector,
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }
}

impl<T> LatestBlock for Cache<T>
where
    T: LatestBlock<Block = Block, BlockHash = Hash> + Clone,
{
    type Block = Block;
    type BlockHash = Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = Arc::clone(&self.cache);
        let mut connector = self.connector.clone();

        let future = async move {
            let block = connector.latest_block().compat().await?;

            let block_hash = block.bitcoin_hash();
            let mut guard = cache.lock().await;
            if guard.get(&block_hash).is_none() {
                guard.put(block_hash, block.clone());
            }

            Ok(block)
        }
        .boxed()
        .compat();

        Box::new(future)
    }
}

impl<T> BlockByHash for Cache<T>
where
    T: BlockByHash<Block = Block, BlockHash = Hash> + Clone,
{
    type Block = Block;
    type BlockHash = Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let cache = Arc::clone(&self.cache);
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
