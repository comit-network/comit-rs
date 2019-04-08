#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate debug_stub_derive;

mod api;
pub mod client;
pub mod config;
pub mod connection;

pub mod json;
pub mod shutdown_handle;

pub use crate::api::*;
