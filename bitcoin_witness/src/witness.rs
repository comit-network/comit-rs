use bitcoin_support::Script;
use secp256k1_support::{PublicKey, SecretKey};

#[derive(Clone, Debug)]
pub enum Witness {
    Data(Vec<u8>),
    Signature(SecretKey),
    PublicKey(PublicKey),
    Bool(bool),
    PrevScript,
}

pub trait WitnessMethod {
    fn into_witness(self) -> Vec<Witness>;
    fn sequence(&self) -> u32;
    fn prev_script(&self) -> Script;
}
