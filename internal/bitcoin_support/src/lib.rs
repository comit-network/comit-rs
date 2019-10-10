#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use crate::{blocks::*, network::*, pubkey::*, transaction::*};
pub use bitcoin::{
    blockdata::{
        block::{Block, BlockHeader},
        opcodes,
        script::{self, Script},
        transaction::{OutPoint, SigHashType, Transaction, TxIn, TxOut},
    },
    consensus::{self, deserialize, encode::serialize_hex, serialize},
    hashes::{
        hash160::Hash as Hash160, hex::FromHex, sha256d::Hash as Sha256dHash, Error as HashesError,
        Hash,
    },
    util::{
        amount,
        bip143::SighashComponents,
        bip32::{self, ChainCode, ChildNumber, ExtendedPrivKey, ExtendedPubKey, Fingerprint},
        hash::BitcoinHash,
        key::{PrivateKey, PublicKey},
        Error,
    },
    Address, Amount,
};
pub use secp_wrapper;
pub use Sha256dHash as TransactionId;
pub use Sha256dHash as BlockId;

mod blocks;
mod network;
mod pubkey;
mod transaction;
