use crate::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{
        web3::{
            self,
            transports::{EventLoopHandle, Http},
            Web3,
        },
        BlockId, BlockNumber,
    },
};
use futures::Future;
use futures_core::{compat::Future01CompatExt, TryFutureExt};
use reqwest::Url;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct Web3Connector {
    web3: Arc<Web3<Http>>,
    task_executor: tokio::runtime::TaskExecutor,
    block_cache:
        HashMap<crate::ethereum::H256, crate::ethereum::Block<crate::ethereum::Transaction>>,
}

impl Web3Connector {
    pub fn new(
        node_url: Url,
        task_executor: tokio::runtime::TaskExecutor,
    ) -> Result<(Self, EventLoopHandle), web3::Error> {
        let (event_loop_handle, http_transport) = Http::new(node_url.as_str())?;
        let block_cache: HashMap<
            crate::ethereum::H256,
            crate::ethereum::Block<crate::ethereum::Transaction>,
        > = HashMap::new();
        Ok((
            Self {
                web3: Arc::new(Web3::new(http_transport)),
                task_executor,
                block_cache,
            },
            event_loop_handle,
        ))
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
        &mut self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(Box::pin(block_by_hash(self.clone(), block_hash)).compat())
    }
}

async fn block_by_hash(
    mut connector: Web3Connector,
    block_hash: crate::ethereum::H256,
) -> Result<
    Option<crate::ethereum::Block<crate::ethereum::Transaction>>,
    crate::ethereum::web3::Error,
> {
    if let Some(block) = connector.block_cache.get(&block_hash) {
        log::trace!("Found block in cache: {:?}", block);
        return Ok(Some(block.clone()));
    }

    let web = connector.web3.clone();
    let block = web
        .eth()
        .block_with_txs(BlockId::Hash(block_hash))
        .compat()
        .await?;

    match block {
        Some(block) => {
            log::trace!("Fetched block from web3 connector: {:?}", block);

            connector.block_cache.insert(block_hash, block.clone());
            Ok(Some(block))
        }
        None => Ok(None),
    }
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
