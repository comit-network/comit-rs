#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

extern crate bitcoin_support;
extern crate secp256k1_support;

// https://github.com/bitcoin/bips/blob/master/bip-0125.mediawiki
// Wallets that don't want to signal replaceability should use either a
// max sequence number (0xffffffff) or a sequence number of
//(0xffffffff-1) when then also want to use locktime;
pub const SEQUENCE_ALLOW_NTIMELOCK_NO_RBF: u32 = 0xFFFF_FFFE;
#[allow(dead_code)]
pub const SEQUENCE_DISALLOW_NTIMELOCK_NO_RBF: u32 = 0xFFFF_FFFF;

mod p2wpkh;
mod primed_transaction;
mod witness;

pub use p2wpkh::*;
pub use primed_transaction::*;
pub use witness::*;
