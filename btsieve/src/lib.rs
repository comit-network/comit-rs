#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod blocksource;
pub mod ethereum;
pub mod matching_transactions;

pub use ethereum_support::web3;
use std::cmp::Ordering;

#[derive(PartialEq, PartialOrd)]
pub struct QueryId(pub String);

#[derive(PartialEq)]
pub struct QueryMatch(pub QueryId, pub String);

impl From<String> for QueryId {
    fn from(item: String) -> Self {
        Self(item)
    }
}

impl PartialOrd for QueryMatch {
    fn partial_cmp(&self, other: &QueryMatch) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

pub trait IntoTransactionId: 'static {
    fn into_transaction_id(&self) -> String;
}

impl IntoTransactionId for ethereum_support::Transaction {
    fn into_transaction_id(&self) -> String {
        format!("{:x}", self.hash)
    }
}

impl IntoTransactionId for bitcoin_support::Transaction {
    fn into_transaction_id(&self) -> String {
        format!("{:x}", self.txid())
    }
}
