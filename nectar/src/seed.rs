use ::bitcoin::{
    hashes::{sha512, Hash, HashEngine, Hmac, HmacEngine},
    secp256k1::{self, constants::SECRET_KEY_SIZE, SecretKey},
};
use rand::prelude::*;
use std::fmt;

pub const SEED_LENGTH: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Seed([u8; SEED_LENGTH]);

impl Seed {
    pub fn random() -> Result<Self, Error> {
        let mut bytes = [0u8; SECRET_KEY_SIZE];
        rand::thread_rng().fill_bytes(&mut bytes);

        // If it succeeds once, it'll always succeed
        let _ = SecretKey::from_slice(&bytes)?;

        Ok(Seed(bytes))
    }

    pub fn bytes(&self) -> [u8; SEED_LENGTH] {
        self.0
    }

    /// Return the private key and chain code to be used as root extended
    /// private key for a BIP32 wallet.
    pub fn root_secret_key_chain_code(&self) -> (SecretKey, Vec<u8>) {
        let bytes = self.bytes();

        // Yes, this is as per BIP32 and used in both Bitcoin and Ethereum ecosystems
        let hash_key = b"Bitcoin seed";

        let mut engine = HmacEngine::<sha512::Hash>::new(hash_key);
        engine.input(&bytes);
        let hash = Hmac::<sha512::Hash>::from_engine(engine);
        let output = &hash.into_inner()[..];
        let key = &output[..32];
        let chain_code = &output[32..];

        let secret_key = SecretKey::from_slice(key).expect("32 bytes array should be fine");

        (secret_key, chain_code.to_vec())
    }

    /// Do note that the secret key returned only contains the seed bytes.
    /// This helper function provides a different format but does not
    /// manipulate the seed. Further computation may be needed to match
    /// the practice of the given blockchain
    pub fn as_secret_key(&self) -> SecretKey {
        SecretKey::from_slice(&self.0).expect("It worked in ::random()")
    }
}

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed([*****])")
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<[u8; SEED_LENGTH]> for Seed {
    fn from(bytes: [u8; SEED_LENGTH]) -> Self {
        Seed(bytes)
    }
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum Error {
    #[error("Secp256k1: ")]
    Secp256k1(#[from] secp256k1::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let _ = Seed::random().unwrap();
    }
}
