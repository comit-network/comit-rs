#![feature(plugin, decl_macro)]

extern crate bitcoin;
extern crate common_types;
extern crate hex;

pub mod btc_htlc;

pub use btc_htlc::*;
pub use common_types::secret;
