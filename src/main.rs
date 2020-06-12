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

mod bitcoin_wallet;
mod bitcoind;
mod jsonrpc;
mod ongoing_swaps;
mod publish;

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

fn main() {
    println!("Hello, world!");
}
