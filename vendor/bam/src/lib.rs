#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

mod api;
// pub mod client;
// pub mod config;
// pub mod connection;

#[macro_use]
pub mod json;
// pub mod shutdown_handle;

pub use crate::api::*;
