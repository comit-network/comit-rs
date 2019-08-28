#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use crate::{blocks::*, mined_block::*, network::*, pubkey::*, transaction::*};
pub use bitcoin::{
    blockdata::{
        block::{Block, BlockHeader},
        opcodes,
        script::{self, Script},
        transaction::{OutPoint, SigHashType, Transaction, TxIn, TxOut},
    },
    consensus::{deserialize, encode::serialize_hex, serialize},
    hashes::{hash160::Hash as Hash160, hex::FromHex, sha256d::Hash as Sha256dHash, Hash},
    secp256k1,
    util::{
        amount,
        bip143::SighashComponents,
        bip32::{self, ChainCode, ChildNumber, ExtendedPrivKey, ExtendedPubKey, Fingerprint},
        hash::BitcoinHash,
        key::PrivateKey,
        Error,
    },
    Address,
};
pub use bitcoin_quantity::*;
pub use Sha256dHash as TransactionId;
pub use Sha256dHash as BlockId;

mod blocks;
mod mined_block;
mod network;
mod pubkey;
mod transaction;
