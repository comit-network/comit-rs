#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

#[macro_use]
extern crate serde_derive;

use reqwest;

pub mod api_client;
