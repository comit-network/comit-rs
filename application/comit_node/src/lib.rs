#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![cfg_attr(test, deny(warnings))]

extern crate bitcoin_rpc_client;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate tokio;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_htlc;
extern crate bitcoin_support;
extern crate bitcoin_witness;
extern crate common_types;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate secp256k1_support;
extern crate serde_json;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate event_store;
#[macro_use]
extern crate futures;
extern crate ganp;
extern crate hex;
extern crate logging;
extern crate rand;
extern crate rustc_hex;
extern crate transport_protocol;

#[cfg(test)]
extern crate env_logger;
#[cfg(test)]
extern crate spectral;

pub mod bitcoin_fee_service;
pub mod bitcoin_payment_future;
pub mod comit_node_api_client;
pub mod comit_server;
pub mod futures_ext;
pub mod gas_price_service;
pub mod ledger_query_service;
pub mod rocket_factory;
pub mod swap_protocols;
pub mod swaps;
