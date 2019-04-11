#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

pub use crate::{
    contract_address::*, erc20_quantity::*, erc20_token::*, ether_quantity::*, key::*, network::*,
    u256_ext::*,
};
pub use extern_web3::{futures::Future, types::*};

mod contract_address;
mod erc20_quantity;
mod erc20_token;
mod ether_quantity;
mod key;
mod network;
mod u256_ext;

pub mod web3 {
    pub use extern_web3::{
        api,
        error::{Error, ErrorKind},
        futures, types,
    };

    pub use extern_web3::Web3;

    pub mod transports {
        pub use extern_web3::transports::{EventLoopHandle, Http};
    }
}

#[derive(Debug)]
pub struct TransactionAndReceipt {
    pub transaction: Transaction,
    pub receipt: TransactionReceipt,
}

/// Return `s` without the `0x` at the beginning of it, if any.
// Taken from https://docs.rs/fixed-hash/0.2.2/fixed_hash/
pub fn clean_0x(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    }
}
