// https://github.com/bitcoin/bips/blob/master/bip-0125.mediawiki
// Wallets that don't want to signal replaceability should use either a
// max sequence number (0xffffffff) or a sequence number of
//(0xffffffff-1) when then also want to use locktime;
pub const SEQUENCE_ALLOW_NTIMELOCK_NO_RBF: u32 = 0xFFFF_FFFE;
#[allow(dead_code)]
pub const SEQUENCE_DISALLOW_NTIMELOCK_NO_RBF: u32 = 0xFFFF_FFFF;

mod p2wpkh;
mod primed_transaction;
mod pubkey_hash;

pub use p2wpkh::UnlockP2wpkh;
pub use primed_transaction::{Error, PrimedInput, PrimedTransaction};
pub use pubkey_hash::PubkeyHash;

use rust_bitcoin::{
    secp256k1::{PublicKey, SecretKey},
    Script,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Witness {
    Data(Vec<u8>),
    Signature(SecretKey),
    PublicKey(PublicKey),
    Bool(bool),
    PrevScript,
}

/// In order to properly describe how to unlock an output you need
/// to know several things:
/// * The witness data (which produces the unlocking script)
/// * The sequence number (which has to match the `prev_script` in the case of
///   CHECKSEQUENCEVERIFY)
/// * The `prev_script` of the output you're unlocking
/// This trait may add more things to this list in the future (such as
/// the locktime the transaction must use to pass CHECKLOCKTIMEVERIFY).
#[derive(Debug, Clone, PartialEq)]
pub struct UnlockParameters {
    pub witness: Vec<Witness>,
    pub sequence: u32,
    pub locktime: u32,
    pub prev_script: Script,
}
