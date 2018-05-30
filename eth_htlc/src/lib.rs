extern crate hex;
extern crate web3;
#[macro_use]
extern crate log;
extern crate chrono;

mod htlc;

pub use htlc::*;
pub type SecretHash = web3::types::H256;
pub use web3::types::Address;
