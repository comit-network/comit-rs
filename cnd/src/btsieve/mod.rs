#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

#[macro_use]
mod block_cache;

pub mod bitcoin;
pub mod ethereum;

use async_trait::async_trait;
use tokio::prelude::{Future, Stream};

pub trait MatchingTransactions<P>: Send + Sync + 'static {
    type Transaction;

    fn matching_transactions(
        &self,
        pattern: P,
        timestamp: Option<u32>,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send>;
}

pub trait LatestBlock: Send + Sync + 'static {
    type Error: std::fmt::Debug;
    type Block;
    type BlockHash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static>;
}

pub trait BlockByHash: Send + Sync + 'static {
    type Error: std::fmt::Debug;
    type Block;
    type BlockHash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static>;
}

#[async_trait]
pub trait BlockCache: Send + Sync + 'static {
    type Block;
    type BlockHash;

    async fn get(&self, block_hash: &Self::BlockHash) -> anyhow::Result<Option<Self::Block>>;
    async fn insert(
        &mut self,
        block_hash: Self::BlockHash,
        block: Self::Block,
    ) -> anyhow::Result<Option<Self::Block>>;
}

pub trait ReceiptByHash: Send + Sync + 'static {
    type Receipt;
    type TransactionHash;
    type Error: std::fmt::Debug;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = Self::Error> + Send + 'static>;
}
