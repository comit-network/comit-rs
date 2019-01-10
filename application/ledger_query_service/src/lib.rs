#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

pub mod bitcoin;
pub mod ethereum;
mod in_memory_query_repository;
mod in_memory_query_result_repository;
mod query_repository;
mod query_result_repository;
pub mod route_factory;
mod routes;
pub mod settings;

pub use crate::{
    in_memory_query_repository::*, in_memory_query_result_repository::*, query_repository::*,
    query_result_repository::*, route_factory::*, routes::*,
};
pub use ethereum_support::web3;
use futures::Future;
use std::fmt::Debug;

type QueryMatch = (u32, String);

pub trait BlockProcessor<B> {
    fn process(
        &mut self,
        block: B,
    ) -> Box<dyn Future<Item = (Vec<QueryMatch>, Vec<QueryMatch>), Error = ()> + Send>;
}

pub trait Query<O>: Debug + 'static {
    fn matches(&self, object: &O) -> Box<dyn Future<Item = QueryMatchResult, Error = ()> + Send>;
    fn is_empty(&self) -> bool;
}

#[derive(Debug, PartialEq)]
pub enum QueryMatchResult {
    Yes { confirmations_needed: u32 },
    No,
}

impl QueryMatchResult {
    pub fn yes() -> Self {
        QueryMatchResult::Yes {
            confirmations_needed: 0,
        }
    }
    pub fn yes_with_confirmations(confirmations_needed: u32) -> Self {
        QueryMatchResult::Yes {
            confirmations_needed,
        }
    }
    pub fn no() -> Self {
        QueryMatchResult::No
    }
}
