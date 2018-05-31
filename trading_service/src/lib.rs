#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate hex;
extern crate lazy_static;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

extern crate crypto;
extern crate rand;

#[macro_use]
extern crate log;
extern crate bitcoin;
extern crate serde;

pub mod exchange_api_client;

pub mod btc_htlc;
pub mod event_store;
pub mod rocket_factory;
pub mod routes;
pub mod secret;
pub mod stub;
pub mod symbol;
