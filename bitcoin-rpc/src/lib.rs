#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate jsonrpc;
extern crate serde;
extern crate rustc_serialize;

pub mod types;
pub use types::*;

pub mod bitcoincore;

pub use rustc_serialize::hex;