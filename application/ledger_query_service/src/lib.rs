#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

mod block_processor;
mod connectors;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
mod queries;
mod query_repository;
mod query_result_repository;
pub mod route_factory;
mod routes;
pub mod settings;

pub use crate::{
    block_processor::*,
    connectors::{bitcoind_zmq_listener::*, ethereum_web3_block_poller::*},
    in_memory_query_repository::*,
    in_memory_query_result_repository::*,
    queries::*,
    query_repository::*,
    query_result_repository::*,
    route_factory::*,
    routes::*,
};
pub use ethereum_support::web3;
