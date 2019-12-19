use cnd::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{Block, Transaction, TransactionReceipt, H256},
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::prelude::{Future, IntoFuture};

#[derive(Clone)]
pub struct EthereumConnectorMock {
    all_blocks: HashMap<H256, Block<Transaction>>,
    latest_blocks: Vec<Block<Transaction>>,
    receipts: HashMap<H256, TransactionReceipt>,
    latest_time_return_block: Instant,
    current_latest_block_index: usize,
    task_executor: tokio::runtime::TaskExecutor,
}

impl EthereumConnectorMock {
    pub fn new(
        latest_blocks: impl IntoIterator<Item = Block<Transaction>>,
        all_blocks: impl IntoIterator<Item = Block<Transaction>>,
        receipts: Vec<(H256, TransactionReceipt)>,
        task_executor: tokio::runtime::TaskExecutor,
    ) -> Self {
        EthereumConnectorMock {
            all_blocks: all_blocks
                .into_iter()
                .fold(HashMap::new(), |mut hm, block| {
                    hm.insert(block.hash.unwrap(), block);
                    hm
                }),
            latest_blocks: latest_blocks.into_iter().collect(),
            latest_time_return_block: Instant::now(),
            current_latest_block_index: 0,
            receipts: receipts.into_iter().collect(),
            task_executor,
        }
    }
}

impl LatestBlock for EthereumConnectorMock {
    type Error = String;
    type Block = Option<Block<Transaction>>;
    type BlockHash = H256;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        if self.latest_blocks.is_empty() {
            return Box::new(Err("empty".to_string()).into_future());
        }

        let latest_block = self.latest_blocks[self.current_latest_block_index].clone();
        if self.latest_time_return_block.elapsed() >= Duration::from_secs(1) {
            self.latest_time_return_block = Instant::now();
            if self
                .latest_blocks
                .get(self.current_latest_block_index + 1)
                .is_some()
            {
                self.current_latest_block_index += 1;
            }
        }
        Box::new(Ok(Some(latest_block)).into_future())
    }
}

impl BlockByHash for EthereumConnectorMock {
    type Error = String;
    type Block = Option<Block<Transaction>>;
    type BlockHash = H256;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(Ok(self.all_blocks.get(&block_hash).cloned()).into_future())
    }
}

impl ReceiptByHash for EthereumConnectorMock {
    type Error = String;
    type Receipt = Option<TransactionReceipt>;
    type TransactionHash = H256;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = Self::Error> + Send + 'static> {
        Box::new(Ok(self.receipts.get(&transaction_hash).cloned()).into_future())
    }
}

impl tokio::executor::Executor for EthereumConnectorMock {
    fn spawn(
        &mut self,
        future: Box<dyn Future<Item = (), Error = ()> + Send>,
    ) -> Result<(), tokio::executor::SpawnError> {
        tokio::executor::Executor::spawn(&mut self.task_executor, future)
    }
}
