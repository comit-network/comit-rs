#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(const_fn)]
extern crate bigdecimal;
extern crate bitcoin;
extern crate hex;
extern crate secp256k1_support;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_bech32;
#[cfg(test)]
extern crate spectral;

pub use bitcoin::{
    blockdata::{
        block::{Block, BlockHeader},
        opcodes,
        script::{self, Script},
        transaction::{OutPoint, SigHashType, Transaction, TxIn, TxOut},
    },
    network::{constants::Network, serialize},
    util::{
        bip143::SighashComponents,
        bip32::{self, ChainCode, ChildNumber, ExtendedPrivKey, ExtendedPubKey, Fingerprint},
        hash::{Hash160, Sha256dHash, Sha256dHash as TransactionId, Sha256dHash as BlockHash},
        privkey::Privkey as PrivateKey,
        Error,
    },
    Address,
};

pub use bitcoin_quantity::*;
pub use blocks::*;
pub use pubkey::*;
pub use transaction::*;
pub use weight::*;

mod bitcoin_quantity;
mod blocks;
mod pubkey;
mod transaction;
mod weight;
