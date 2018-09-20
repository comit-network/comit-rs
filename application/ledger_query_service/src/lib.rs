#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_support;
extern crate ethereum_support;
extern crate http_api_problem;
extern crate rocket;
extern crate rocket_contrib;
#[cfg(test)]
extern crate spectral;
#[macro_use]
extern crate log;
extern crate futures;
extern crate web3;
extern crate zmq_rs as zmq;

mod bitcoind_zmq_listener;
mod ethereum_web3_block_poller;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
mod link_factory;
mod query_repository;
mod query_result_repository;
mod routes;
pub mod server_builder;
mod transaction_processor;

pub use bitcoind_zmq_listener::*;
pub use ethereum_web3_block_poller::*;
//TODO: remove web3 dependency
//pub use ethereum_support::web3;
pub use in_memory_query_repository::*;
pub use in_memory_query_result_repository::*;
pub use link_factory::*;
pub use query_repository::*;
pub use query_result_repository::*;
pub use routes::*;
pub use transaction_processor::*;
