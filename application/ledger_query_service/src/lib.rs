#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
#[cfg(test)]
extern crate spectral;

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate byteorder;
extern crate config;
extern crate ethereum_support;
extern crate futures;
extern crate hex;
extern crate http_api_problem;
extern crate hyper;
extern crate serde;
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

pub use crate::{
    bitcoind_zmq_listener::*, block_processor::*, ethereum_web3_block_poller::*,
    in_memory_query_repository::*, in_memory_query_result_repository::*, ledgers::*,
    query_repository::*, query_result_repository::*, route_factory::*, routes::*,
};
pub use ethereum_support::web3;
