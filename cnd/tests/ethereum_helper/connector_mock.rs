use cnd::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{Block, Transaction, TransactionReceipt, H256},
};
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
        }
    }
}

#[async_trait::async_trait]
impl LatestBlock for EthereumConnectorMock {
    type Error = ();
    type Block = Option<Block<Transaction>>;
    type BlockHash = H256;

    async fn latest_block(&mut self) -> Result<Self::Block, Self::Error> {
        if self.latest_blocks.is_empty() {
            return Err(());
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

        Ok(Some(latest_block))
    }
}

#[async_trait::async_trait]
impl BlockByHash for EthereumConnectorMock {
    type Error = ();
    type Block = Option<Block<Transaction>>;
    type BlockHash = H256;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block, Self::Error> {
        Ok(self.all_blocks.get(&block_hash).cloned())
    }
}

#[async_trait::async_trait]
impl ReceiptByHash for EthereumConnectorMock {
    type Receipt = Option<TransactionReceipt>;
    type TransactionHash = H256;
    type Error = ();

    async fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Result<Self::Receipt, Self::Error> {
        Ok(self.receipts.get(&transaction_hash).cloned())
    }
}
