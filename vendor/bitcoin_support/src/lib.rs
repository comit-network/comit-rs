#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

pub use bitcoin::{
    blockdata::{
        block::{Block, BlockHeader},
        opcodes,
        script::{self, Script},
        transaction::{OutPoint, SigHashType, Transaction, TxIn, TxOut},
    },
    consensus::{deserialize, encode::serialize_hex, serialize},
    util::{
        bip143::SighashComponents,
        bip32::{self, ChainCode, ChildNumber, ExtendedPrivKey, ExtendedPubKey, Fingerprint},
        hash::BitcoinHash,
        key::PrivateKey,
        Error,
    },
    Address,
};

pub use bitcoin_hashes::{
    hash160::Hash as Hash160, hex::FromHex, sha256d::Hash as Sha256dHash, Hash,
};
pub use Sha256dHash as TransactionId;
pub use Sha256dHash as BlockId;

pub use crate::{
    blocks::*,
    mined_block::*,
    network::*,
    pubkey::*,
    transaction::*,
    weight::{Error as WeightError, *},
};
pub use bitcoin_quantity::*;

mod blocks;
mod mined_block;
mod network;
mod pubkey;
mod transaction;
mod weight;
