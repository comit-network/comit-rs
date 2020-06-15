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

use conquer_once::Lazy;

pub mod bitcoin;
pub mod bitcoin_wallet;
pub mod bitcoind;
pub mod dai;
pub mod ethereum_wallet;
pub mod float_maths;
pub mod geth;
pub mod jsonrpc;
pub mod ongoing_swaps;
pub mod publish;
pub mod rate;
pub mod seed;
pub mod swap;

pub use seed::Seed;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;
