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

use conquer_once::Lazy;

mod bitcoin;
mod bitcoin_wallet;
mod bitcoind;
mod dai;
mod float_maths;
mod jsonrpc;
mod markets;
mod ongoing_swaps;
mod publish;
mod rate;
mod swap;

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);

fn main() {
    println!("Hello, world!");
}
