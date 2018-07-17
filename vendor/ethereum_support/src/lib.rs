extern crate bigdecimal;
extern crate byteorder;
extern crate num;
extern crate regex;
extern crate secp256k1_support;
extern crate tiny_keccak;
extern crate web3 as web3_crate;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

pub use web3_crate::types::*;

mod ethereum_quantity;
mod key;
mod web3_client;

pub use ethereum_quantity::*;
pub use key::*;
pub use web3_client::*;

pub use web3_crate::futures::Future;

pub mod web3 {
    pub use web3_crate::error::{Error, ErrorKind};

    pub mod transports {
        pub use web3_crate::transports::Http;
    }
}
