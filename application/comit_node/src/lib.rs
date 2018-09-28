#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![feature(tool_lints)]

#[macro_use]
extern crate debug_stub_derive;
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
extern crate comit_wallet;
extern crate config;
extern crate event_store;
extern crate futures;
extern crate hex;
extern crate logging;
extern crate rand;
extern crate rustc_hex;
#[macro_use]
extern crate transport_protocol;
extern crate web3;

#[cfg(test)]
extern crate spectral;

pub mod bitcoin_fee_service;
pub mod comit_node_api_client;
pub mod comit_server;
pub mod gas_price_service;
pub mod rocket_factory;
pub mod settings;
pub mod swap_protocols;
pub mod swaps;
