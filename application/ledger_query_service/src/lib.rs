#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate http_api_problem;
extern crate rocket;
extern crate rocket_contrib;

extern crate bitcoin_support;
#[cfg(test)]
extern crate spectral;

#[macro_use]
extern crate log;

mod in_memory_query_repository;
mod in_memory_query_result_repository;
mod link_factory;
mod query_repository;
mod query_result_repository;
mod routes;
pub mod server;
mod transaction_processor;

pub use in_memory_query_repository::*;
pub use in_memory_query_result_repository::*;
pub use link_factory::*;
pub use query_repository::*;
pub use query_result_repository::*;
pub use routes::*;
pub use transaction_processor::*;
