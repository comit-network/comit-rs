#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

pub mod api_client;
