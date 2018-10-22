#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(custom_attribute)]
#![feature(const_fn)]
extern crate serde;

mod trading_symbol;
pub use trading_symbol::TradingSymbol;
pub mod seconds;
