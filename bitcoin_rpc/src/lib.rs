extern crate bitcoin;
extern crate hex as std_hex;
extern crate jsonrpc;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod bitcoin_rpc_api;
mod bitcoincore;
mod stub_rpc_client;
mod types;

// TODO Figure out, if we can somehow not ship this by default.
extern crate testcontainers;
pub mod test_support;

pub use bitcoin_rpc_api::*;
pub use bitcoincore::*;
pub use jsonrpc::RpcError;
pub use rustc_serialize::hex;
pub use stub_rpc_client::*;
pub use types::*;
