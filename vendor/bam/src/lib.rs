#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate bytes;
extern crate futures;
extern crate tokio;
extern crate tokio_codec;
#[macro_use]
extern crate debug_stub_derive;

#[cfg(test)]
extern crate spectral;

mod api;
pub mod client;
pub mod config;
pub mod connection;
pub mod json;
pub mod shutdown_handle;

pub use crate::api::*;
