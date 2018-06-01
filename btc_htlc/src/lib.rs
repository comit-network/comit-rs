#![feature(plugin, decl_macro)]

extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate common_types;
extern crate hex;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod btc_htlc;

pub use btc_htlc::*;
pub use common_types::secret;
