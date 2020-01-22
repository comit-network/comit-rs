#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod ethereum;

use futures::Future;

pub trait LatestBlock: Send + Sync + 'static {
    type Block;
    type BlockHash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static>;
}

pub trait BlockByHash: Send + Sync + 'static {
    type Block;
    type BlockHash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static>;
}

pub trait ReceiptByHash: Send + Sync + 'static {
    type Receipt;
    type TransactionHash;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = anyhow::Error> + Send + 'static>;
}

/// Checks if a given block predates a certain timestamp.
pub trait Predates {
    fn predates(&self, timestamp: i64) -> bool;
}
