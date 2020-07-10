use ::bitcoin::secp256k1;
use ::bitcoin::secp256k1::constants::SECRET_KEY_SIZE;
use rand::prelude::*;
use sha2::{Digest, Sha256};
use std::fmt;

pub const SEED_LENGTH: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Seed([u8; SEED_LENGTH]);

impl Seed {
    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        let bytes = self.secret_key_bytes();

        Ok(secp256k1::SecretKey::from_slice(&bytes)?)
    }

    /// The secret key is a SHA-256 of the seed
    pub fn secret_key_bytes(&self) -> [u8; SECRET_KEY_SIZE] {
        let mut sha = Sha256::new();
        sha.update(&self.0);

        sha.finalize().into()
    }

    pub fn seed_bytes(&self) -> [u8; SEED_LENGTH] {
        self.0
    }
}

impl Default for Seed {
    fn default() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let seed = Seed::default();
        let res = seed.secret_key();

        assert!(res.is_ok())
    }
}
