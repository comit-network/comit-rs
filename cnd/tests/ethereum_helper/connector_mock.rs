use cnd::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{Block, Transaction, TransactionReceipt, H256},
};
use futures::{future::IntoFuture, Future};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct EthereumConnectorMock {
    all_blocks: HashMap<H256, Block<Transaction>>,
    latest_blocks: Vec<Block<Transaction>>,
    receipts: HashMap<H256, TransactionReceipt>,
    latest_time_return_block: Instant,
    current_latest_block_index: usize,
}

impl EthereumConnectorMock {
    pub fn new(
        latest_blocks: impl IntoIterator<Item = Block<Transaction>>,
        all_blocks: impl IntoIterator<Item = Block<Transaction>>,
        receipts: Vec<(H256, TransactionReceipt)>,
    ) -> Self {
        let all_blocks = all_blocks
            .into_iter()
            .fold(HashMap::new(), |mut hm, block| {
                hm.insert(block.hash.unwrap(), block);
                hm
            });

        EthereumConnectorMock {
            all_blocks,
            latest_blocks: latest_blocks.into_iter().collect(),
            latest_time_return_block: Instant::now(),
            current_latest_block_index: 0,
            receipts: receipts.into_iter().collect(),
        }
    }
}

impl LatestBlock for EthereumConnectorMock {
    type Block = Option<Block<Transaction>>;
    type BlockHash = H256;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        if self.latest_blocks.is_empty() {
            return Box::new(Err(anyhow::Error::from(Error::NoMoreBlocks)).into_future());
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
    type Block = Option<Block<Transaction>>;
    type BlockHash = H256;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        Box::new(Ok(self.all_blocks.get(&block_hash).cloned()).into_future())
    }
}

impl ReceiptByHash for EthereumConnectorMock {
    type Receipt = Option<TransactionReceipt>;
    type TransactionHash = H256;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = anyhow::Error> + Send + 'static> {
        Box::new(Ok(self.receipts.get(&transaction_hash).cloned()).into_future())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ran out of blocks in chain")]
    NoMoreBlocks,
}
