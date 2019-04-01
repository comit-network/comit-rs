#![warn(unused_extern_crates, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
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
