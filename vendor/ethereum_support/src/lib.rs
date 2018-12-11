#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

pub use crate::{contract_address::*, erc20_quantity::*, ether_quantity::*, key::*, u256_ext::*};
pub use ::web3::{futures::Future, types::*};

mod contract_address;
mod erc20_quantity;
mod ether_quantity;
mod key;
mod u256_ext;

pub mod web3 {
    pub use ::web3::{
        api,
        error::{Error, ErrorKind},
        futures, types,
    };

    pub use ::web3::Web3;

    pub mod transports {
        pub use ::web3::transports::{EventLoopHandle, Http};
    }
}
