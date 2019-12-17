use crate::{
    btsieve::{BlockByHash, BlockCache, LatestBlock, ReceiptByHash},
    ethereum::{
        web3::{
            self,
            transports::{EventLoopHandle, Http},
            Web3,
        },
        BlockId, BlockNumber,
    },
};
use async_std::sync::Mutex;
use async_trait::async_trait;
use futures::Future;
use futures_core::{compat::Future01CompatExt, TryFutureExt};
use lru_cache::LruCache;
use reqwest::Url;
use std::sync::Arc;

type Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>;
type Hash = crate::ethereum::H256;

#[derive(Clone, Debug)]
pub struct Web3Connector {
    web3: Arc<Web3<Http>>,
    task_executor: tokio::runtime::TaskExecutor,
    block_cache: Web3BlockCache,
}

impl Web3Connector {
    pub fn new(
        node_url: Url,
        task_executor: tokio::runtime::TaskExecutor,
        cache_capacity: usize,
    ) -> Result<(Self, EventLoopHandle), web3::Error> {
        let (event_loop_handle, http_transport) = Http::new(node_url.as_str())?;
        Ok((
            Self {
                web3: Arc::new(Web3::new(http_transport)),
                task_executor,
                block_cache: Web3BlockCache::new(cache_capacity),
            },
            event_loop_handle,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct Web3BlockCache {
    map: Arc<Mutex<LruCache<Hash, Block>>>,
}

impl Web3BlockCache {
    fn new(capacity: usize) -> Self {
        let map: LruCache<Hash, Block> = LruCache::new(capacity);
        Self {
            map: Arc::new(Mutex::new(map)),
        }
    }
}

#[async_trait]
impl BlockCache for Web3BlockCache {
    type Block = Block;
    type BlockHash = Hash;

    async fn get(&self, block_hash: &Hash) -> anyhow::Result<Option<Block>> {
        let mut cache = self.map.lock().await;
        Ok(cache.get_mut(block_hash).cloned())
    }

    async fn insert(&mut self, block_hash: Hash, block: Block) -> anyhow::Result<Option<Block>> {
        let mut cache = self.map.lock().await;
        Ok(cache.insert(block_hash, block))
    }
}

impl LatestBlock for Web3Connector {
    type Error = crate::ethereum::web3::Error;
    type Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>;
    type BlockHash = crate::ethereum::H256;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(
            web.eth()
                .block_with_txs(BlockId::Number(BlockNumber::Latest)),
        )
    }
}

impl BlockByHash for Web3Connector {
    type Error = crate::ethereum::web3::Error;
    type Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>;
    type BlockHash = crate::ethereum::H256;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(Box::pin(block_by_hash(self.clone(), block_hash)).compat())
    }
}

async fn block_by_hash(
    connector: Web3Connector,
    block_hash: Hash,
) -> Result<Block, crate::ethereum::web3::Error> {
    let mut cache = connector.block_cache.clone();

    let block = cache.get(&block_hash).await;
    if block.is_ok() {
        if let Some(block) = block.unwrap() {
            log::trace!("Found block in cache: {:?}", block);
            return Ok(block.clone());
        }
    }

    let web = connector.web3.clone();
    let block = web
        .eth()
        .block_with_txs(BlockId::Hash(block_hash))
        .compat()
        .await?;

    log::trace!("Fetched block from web3 connector: {:?}", block);

    let _ = cache.insert(block_hash, block.clone());
    Ok(block)
}

impl ReceiptByHash for Web3Connector {
    type Receipt = Option<crate::ethereum::TransactionReceipt>;
    type TransactionHash = crate::ethereum::H256;
    type Error = crate::ethereum::web3::Error;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().transaction_receipt(transaction_hash))
    }
}

impl tokio::executor::Executor for Web3Connector {
    fn spawn(
        &mut self,
        future: Box<dyn Future<Item = (), Error = ()> + Send>,
    ) -> Result<(), tokio::executor::SpawnError> {
        tokio::executor::Executor::spawn(&mut self.task_executor, future)
    }
}
