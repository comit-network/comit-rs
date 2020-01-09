#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod ethereum;

#[async_trait::async_trait]
pub trait LatestBlock: Send + Sync + 'static {
    type Error: std::fmt::Debug;
    type Block;
    type BlockHash;

    async fn latest_block(&mut self) -> Result<Self::Block, Self::Error>;
}

#[async_trait::async_trait]
pub trait BlockByHash: Send + Sync + 'static {
    type Error: std::fmt::Debug;
    type Block;
    type BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block, Self::Error>;
}

#[async_trait::async_trait]
pub trait ReceiptByHash: Send + Sync + 'static {
    type Receipt;
    type TransactionHash;
    type Error: std::fmt::Debug;

    async fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Result<Self::Receipt, Self::Error>;
}
