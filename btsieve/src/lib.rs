#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod blocksource;
pub mod ethereum;
pub mod expected_version_header;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
pub mod load_settings;
pub mod logging;
mod query_repository;
mod query_result_repository;
pub mod route_factory;
pub mod routes;
pub mod settings;

pub use crate::{
    in_memory_query_repository::*, in_memory_query_result_repository::*, query_repository::*,
    query_result_repository::*, route_factory::*, routes::*,
};
use bitcoin_support::Sha256dHash;
pub use ethereum_support::web3;
use ethereum_support::{Block, Transaction, H256};
use std::{cmp::Ordering, sync::Arc};

#[derive(PartialEq, PartialOrd)]
pub struct QueryId(pub String);

#[derive(PartialEq)]
pub struct QueryMatch(pub QueryId, pub String);

type ArcQueryRepository<Q> = Arc<dyn QueryRepository<Q>>;

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

struct BlockchainDag<T, V> {
    nodes: Vec<T>,
    vertices: Vec<V>,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self(BlockchainDag {
            nodes: Vec::new(),
            vertices: Vec::new(),
        })
    }
}

impl Default for Ethereum {
    fn default() -> Self {
        Self(BlockchainDag {
            nodes: Vec::new(),
            vertices: Vec::new(),
        })
    }
}

pub struct Bitcoin(BlockchainDag<bitcoin_support::Block, (Sha256dHash, Sha256dHash)>);
pub struct Ethereum(BlockchainDag<Block<Transaction>, (H256, H256)>);

pub trait Blockchain<T> {
    fn add_block(&mut self, block: T);
    fn size(&self) -> usize;
    fn find_predecessor(&self, block: &T) -> Option<&T>;
}
