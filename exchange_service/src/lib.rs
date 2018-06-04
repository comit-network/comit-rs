#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate ethereum_htlc;
extern crate serde_json;
extern crate uuid;
extern crate web3;

#[macro_use]
extern crate log;

pub mod ethereum_service;
pub mod event_store;
pub mod gas_price_service;
pub mod rocket_factory;
pub mod routes;
pub mod treasury_api_client;
