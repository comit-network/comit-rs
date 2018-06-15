#![feature(plugin, decl_macro)]

extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate common_types;
extern crate hex;

pub mod bitcoin_htlc;

pub use bitcoin_htlc::*;
pub use common_types::secret;
