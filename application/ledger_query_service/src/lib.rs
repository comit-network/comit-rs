#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(tool_lints)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_support;
extern crate ethereum_support;
extern crate serde;
#[cfg(test)]
extern crate spectral;
#[macro_use]
extern crate log;
extern crate bitcoin_rpc_client;
extern crate byteorder;
extern crate config;
extern crate url;
extern crate warp;
extern crate zmq_rs as zmq;

mod bitcoind_zmq_listener;
mod block_processor;
mod ethereum_web3_block_poller;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
mod ledgers;
mod query_repository;
mod query_result_repository;
pub mod route_factory;
mod routes;
pub mod settings;

pub use bitcoind_zmq_listener::*;
pub use block_processor::*;
pub use ethereum_support::web3;
pub use ethereum_web3_block_poller::*;
pub use in_memory_query_repository::*;
pub use in_memory_query_result_repository::*;
pub use ledgers::*;
pub use query_repository::*;
pub use query_result_repository::*;
pub use route_factory::*;
pub use routes::*;
