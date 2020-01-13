#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod ethereum;

use futures::{Future, Stream};
use std::fmt::Display;

pub trait MatchingTransactions<P>: Send + Sync + 'static {
    type Transaction;

    fn matching_transactions(
        &self,
        pattern: P,
        timestamp: Option<u32>,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send>;
}

pub trait LatestBlock: Send + Sync + 'static {
    type Error: Display;
    type Block;
    type BlockHash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static>;
}

pub trait BlockByHash: Send + Sync + 'static {
    type Error: Display;
    type Block;
    type BlockHash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static>;
}

pub trait ReceiptByHash: Send + Sync + 'static {
    type Receipt;
    type TransactionHash;
    type Error: Display;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = Self::Error> + Send + 'static>;
}
