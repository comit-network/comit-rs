#![warn(
    unused_results,
    unused_extern_crates,
    missing_debug_implementations
)]
#![deny(unsafe_code)]
#![feature(custom_attribute)]
#![feature(const_fn)]
extern crate crypto;
extern crate hex;
extern crate rand;
extern crate serde;

pub mod secret;
mod trading_symbol;
pub use trading_symbol::TradingSymbol;
pub mod seconds;
