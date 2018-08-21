#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_htlc;
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate common_types;
extern crate crypto;
extern crate ethereum_htlc;
extern crate ethereum_support;
extern crate event_store;
extern crate hex;
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate logging;
extern crate rand;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate rustc_hex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

pub use common_types::secret;

pub mod exchange_api_client;

pub mod rocket_factory;
pub mod swaps;
