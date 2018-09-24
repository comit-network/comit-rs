#![feature(const_fn)]
extern crate bigdecimal;
extern crate bitcoin;
extern crate bitcoin_bech32;
extern crate bitcoin_rpc_client;
extern crate hex;
extern crate secp256k1_support;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate spectral;

pub use address::Address;
pub use bitcoin::{
    blockdata::{
        block::Block,
        script::Script,
        transaction::{OutPoint, SigHashType, Transaction, TxIn, TxOut},
    },
    network::{constants::Network, serialize},
    util::{
        bip143::SighashComponents,
        bip32::{ChainCode, ChildNumber, ExtendedPrivKey, Fingerprint},
        hash::{Hash160, Sha256dHash},
        privkey::Privkey as PrivateKey,
        Error,
    },
};
pub use bitcoin_quantity::*;
pub use blocks::*;
pub use pubkey::*;
pub use transaction::*;
pub use weight::*;

mod address;
mod bitcoin_quantity;
mod blocks;
mod pubkey;
mod transaction;
mod weight;
