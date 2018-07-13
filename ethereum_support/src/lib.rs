extern crate secp256k1_support;
extern crate tiny_keccak;
extern crate web3 as web3_crate;

pub use web3_crate::types::*;

mod key;
mod web3_client;

pub use key::*;
pub use web3_client::*;
pub use web3_crate::futures::Future;

pub mod web3 {
    pub use web3_crate::error::Error;
    pub use web3_crate::error::ErrorKind;

    pub mod transports {
        pub use web3_crate::transports::Http;
    }
}
