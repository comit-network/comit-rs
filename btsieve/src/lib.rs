#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod ethereum;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
pub mod load_settings;
pub mod logging;
mod query_repository;
mod query_result_repository;
pub mod route_factory;
mod routes;
pub mod settings;

pub use crate::{
    in_memory_query_repository::*, in_memory_query_result_repository::*, query_repository::*,
    query_result_repository::*, route_factory::*, routes::*,
};
use bitcoin_support::{MinedBlock, Sha256dHash};
pub use ethereum_support::web3;
use std::{cmp::Ordering, sync::Arc};

#[derive(PartialEq, PartialOrd)]
pub struct QueryId(pub u32);

#[derive(PartialEq)]
pub struct QueryMatch(pub QueryId, pub String);

type ArcQueryRepository<Q> = Arc<dyn QueryRepository<Q>>;

impl From<u32> for QueryId {
    fn from(item: u32) -> Self {
        Self(item)
    }
}

impl PartialOrd for QueryMatch {
    fn partial_cmp(&self, other: &QueryMatch) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

struct BlockchainDAG<T, V> {
    nodes: Vec<T>,
    vertices: Vec<V>,
}

impl Default for Bitcoin {
    fn default() -> Self {
        Self(BlockchainDAG {
            nodes: Vec::new(),
            vertices: Vec::new(),
        })
    }
}

struct Bitcoin(BlockchainDAG<MinedBlock, (Sha256dHash, Sha256dHash)>);

struct Ethereum;

trait AddBlock<T> {
    fn add_block(&mut self, block: T);
    fn size(&self) -> usize;
    fn find_predecessor(&self, block: &T) -> Option<&MinedBlock>;
}
