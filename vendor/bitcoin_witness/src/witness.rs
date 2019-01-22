use bitcoin_support::Script;
use secp256k1_support::{KeyPair, PublicKey};

#[derive(Clone, Debug, PartialEq)]
pub enum Witness {
    Data(Vec<u8>),
    Signature(KeyPair),
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
    pub locktime: i64,
    pub prev_script: Script,
}
