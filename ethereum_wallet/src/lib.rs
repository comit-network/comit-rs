extern crate hex;
extern crate rlp;
extern crate secp256k1;
extern crate tiny_keccak;
extern crate web3;

mod transaction;
mod wallet;

pub use transaction::UnsignedTransaction;
pub use wallet::{InMemoryWallet, Wallet};

pub mod fake;
pub mod key;
pub use key::*;
