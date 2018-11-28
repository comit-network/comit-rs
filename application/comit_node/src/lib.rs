#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate enum_display_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate bam;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate state_machine_future;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate frunk;
#[macro_use]
extern crate binary_macros;

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
extern crate fern;
extern crate frunk_core;
extern crate hex;
extern crate http;
extern crate http_api_problem;
extern crate hyper;
extern crate rand;
extern crate reqwest;
extern crate rustc_hex;
extern crate rustic_hal;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate url;
extern crate uuid;
extern crate warp;

pub mod bam_api;
pub mod comit_client;
pub mod comit_server;
pub mod http_api;
pub mod item_cache;
pub mod key_store;
pub mod ledger_query_service;
pub mod logging;
pub mod settings;
pub mod swap_protocols;
pub mod swaps;
