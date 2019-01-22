#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate enum_display_derive;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
#[macro_use]
extern crate bam;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate frunk;
#[macro_use]
extern crate binary_macros;

pub mod bam_api;
pub mod bam_ext;
pub mod comit_client;
pub mod comit_server;
pub mod http_api;
pub mod item_cache;
pub mod ledger_query_service;
pub mod logging;
pub mod seed;
pub mod settings;
pub mod swap_protocols;
