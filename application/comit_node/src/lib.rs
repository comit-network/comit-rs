#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]
#![feature(tool_lints)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate transport_protocol;
#[macro_use]
extern crate futures;

#[cfg(test)]
extern crate pretty_env_logger;
#[cfg(test)]
extern crate spectral;

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate bitcoin_witness;
extern crate chrono;
extern crate config;
extern crate crypto;
extern crate ethereum_support;
extern crate event_store;
extern crate fern;
extern crate gotham;
extern crate hex;
extern crate http_api_problem;
extern crate hyper;
extern crate mime;
extern crate rand;
extern crate reqwest;
extern crate rlp;
extern crate rustc_hex;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;
extern crate tiny_keccak;
extern crate tokio;
extern crate tokio_timer;
extern crate url;
extern crate uuid;
#[macro_use]
extern crate state_machine_future;

pub mod bitcoin_fee_service;

pub mod comit_client;
pub mod comit_server;
pub mod ethereum_wallet;
pub mod gas_price_service;
pub mod gotham_factory;
pub mod http_api;
pub mod item_cache;
pub mod key_store;
pub mod ledger_query_service;
pub mod logging;
pub mod settings;
pub mod swap_protocols;
pub mod swaps;
