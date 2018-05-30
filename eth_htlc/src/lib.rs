extern crate hex;
extern crate web3;

mod htlc;

pub use htlc::*;
pub type SecretHash = web3::types::H256;
pub use web3::types::Address;
