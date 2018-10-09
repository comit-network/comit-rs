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
extern crate tokio_timer;
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
extern crate config;
extern crate event_store;
#[macro_use]
extern crate futures;
extern crate gotham;
extern crate hex;
extern crate http_api_problem;
extern crate logging;
extern crate rand;
extern crate rustc_hex;
#[macro_use]
extern crate transport_protocol;
#[macro_use]
extern crate gotham_derive;
extern crate comit_wallet;
extern crate hyper;
extern crate mime;
#[macro_use]
extern crate failure;
extern crate bip39;
extern crate url;

#[cfg(test)]
extern crate spectral;

pub mod bitcoin_fee_service;

pub mod bitcoin_payment_future;
pub mod comit_client;

pub mod comit_server;
pub mod futures_ext;
pub mod gas_price_service;
pub mod gotham_factory;
pub mod http_api;
pub mod ledger_query_service;
pub mod rocket_factory;
pub mod settings;
pub mod swap_protocols;
pub mod swaps;
