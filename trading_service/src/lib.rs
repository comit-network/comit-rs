#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_htlc;
extern crate bitcoin_rpc;
extern crate common_types;
extern crate hex;
extern crate lazy_static;
extern crate rand;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

extern crate crypto;
extern crate rand;
extern crate web3;

#[macro_use]
extern crate log;
extern crate bitcoin;
extern crate serde;

pub mod exchange_api_client;

pub mod event_store;
pub mod rocket_factory;
pub mod routes;
pub mod stub;
pub mod symbol;

pub use common_types::secret;
