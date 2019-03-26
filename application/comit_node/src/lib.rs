#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
#[macro_use]
extern crate derivative;
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
extern crate binary_macros;
#[macro_use]
extern crate strum_macros;

pub mod bam_api;
pub mod bam_ext;
pub mod btsieve;
pub mod comit_client;
pub mod comit_server;
pub mod connection_pool;
pub mod http_api;
pub mod logging;
pub mod node_id;
pub mod seed;
pub mod settings;
pub mod swap_protocols;
