#![feature(plugin, decl_macro)]

extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate bitcoin_support;
extern crate common_types;
extern crate hex;
#[macro_use]
extern crate log;

pub mod bitcoin_htlc;

pub use bitcoin_htlc::*;
pub use common_types::secret;
