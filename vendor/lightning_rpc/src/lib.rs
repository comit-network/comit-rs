#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

#[macro_use]
extern crate debug_stub_derive;
extern crate futures;
extern crate hex;
extern crate http;
extern crate pem;
#[macro_use]
extern crate prost_derive;
extern crate tls_api;
extern crate tls_api_native_tls;
extern crate tokio;
extern crate tokio_tls_api;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_http;
#[macro_use]
extern crate log;

// Includes the proto generated files
pub mod lnrpc {
    pub use tower_grpc::{Error, Request};
    include!(concat!(env!("OUT_DIR"), "/lnrpc.rs"));
}
pub mod certificate;
pub mod macaroon;

mod add_macaroon;
mod factory;
mod from_file;

pub(crate) use self::add_macaroon::AddMacaroon;

pub use self::{
    factory::{ClientFactory, Error, LndClient},
    from_file::FromFile,
};
