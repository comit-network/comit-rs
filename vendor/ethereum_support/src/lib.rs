#![warn(
    unused_results,
    unused_extern_crates,
    missing_debug_implementations
)]
#![deny(unsafe_code)]

extern crate bigdecimal;
extern crate byteorder;
extern crate num;
extern crate regex;
extern crate secp256k1_support;
extern crate tiny_keccak;
extern crate web3 as web3_crate;

#[macro_use]
extern crate lazy_static;
extern crate serde;

pub use web3_crate::types::*;

mod ethereum_quantity;
mod key;

pub use ethereum_quantity::*;
pub use key::*;

pub use web3_crate::futures::Future;

pub mod web3 {

    pub use web3_crate::{
        api,
        error::{Error, ErrorKind},
        futures, types,
    };

    pub use web3_crate::Web3;
    pub mod transports {
        pub use web3_crate::transports::{EventLoopHandle, Http};
    }
}
