#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

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
extern crate maplit;
#[macro_use]
extern crate frunk;
#[macro_use]
extern crate binary_macros;
extern crate hex_serde;
#[cfg(test)]
extern crate pretty_env_logger;
#[cfg(test)]
extern crate serde_urlencoded;

use bitcoin_rpc_client;
use bitcoin_support;
use chrono;
use ethereum_support;
use frunk_core;
use hex;
use reqwest;
use rustc_hex;
use secp256k1_support;
use serde;
use serde_json;
use tokio;
use url;
use warp;

pub mod bam_api;
pub mod comit_client;
pub mod comit_server;
pub mod http_api;
pub mod item_cache;
pub mod ledger_query_service;
pub mod logging;
pub mod seed;
pub mod settings;
pub mod swap_protocols;
