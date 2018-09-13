#![feature(custom_attribute)]
#![feature(const_fn)]
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate crypto;
extern crate ethereum_support;
extern crate hex;
extern crate rand;
extern crate secp256k1_support;
extern crate serde;

pub mod secret;
mod trading_symbol;
pub use trading_symbol::TradingSymbol;
pub mod seconds;
