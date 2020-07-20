#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::dbg_macro
)]
#![allow(dead_code)] // To be removed further down the line
#![forbid(unsafe_code)]
// TODO: Add no unwrap policy

pub mod bitcoin;
pub mod command;
pub mod config;
pub mod ethereum;
pub mod float_maths;
pub mod fs;
pub mod history;
pub mod jsonrpc;
pub mod maker;
pub mod mid_market_rate;
pub mod network;
pub mod order;
pub mod rate;
pub mod seed;
pub mod swap;
pub mod swap_id;

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

use conquer_once::Lazy;
pub use maker::Maker;
pub use mid_market_rate::MidMarketRate;
pub use rate::{Rate, Spread};
pub use seed::Seed;
pub use swap_id::SwapId;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);
