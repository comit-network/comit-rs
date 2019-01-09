#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

mod bitcoin;
mod block_processor;
mod ethereum;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
mod query_repository;
mod query_result_repository;
pub mod route_factory;
mod routes;
pub mod settings;

pub use crate::{
    bitcoin::{bitcoind_zmq_listener::*, queries::*},
    block_processor::*,
    ethereum::{ethereum_web3_block_poller::*, queries::*},
    in_memory_query_repository::*,
    in_memory_query_result_repository::*,
    query_repository::*,
    query_result_repository::*,
    route_factory::*,
    routes::*,
};
pub use ethereum_support::web3;
