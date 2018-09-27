#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

extern crate bytes;
extern crate futures;
extern crate hex;
extern crate http;
extern crate pem;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tls_api;
extern crate tls_api_native_tls;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls_api;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_http;

use std::path::Path;

// Includes the proto generated files
pub mod lnrpc {
    include!(concat!(env!("OUT_DIR"), "/lnrpc.rs"));
}
pub mod certificate;
pub mod lightning_rpc_api;
pub mod lnd_api;
pub mod macaroon;

pub trait FromFile
where
    Self: std::marker::Sized,
{
    type Err;

    fn from_file<P: AsRef<Path>>(file: P) -> Result<Self, Self::Err>;
}
