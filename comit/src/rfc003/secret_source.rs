use crate::rfc003::Secret;
use secp256k1_keypair::KeyPair;

pub trait SecretSource: Send + Sync {
    fn secret(&self) -> Secret;
    fn secp256k1_redeem(&self) -> KeyPair;
    fn secp256k1_refund(&self) -> KeyPair;
}
