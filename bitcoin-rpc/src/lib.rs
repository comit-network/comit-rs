extern crate jsonrpc;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
pub mod types;
pub use types::*;

pub mod bitcoincore;

pub use rustc_serialize::hex;
