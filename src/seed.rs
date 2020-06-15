use bitcoin::{secp256k1, secp256k1::constants::SECRET_KEY_SIZE};
use rand::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Seed([u8; SECRET_KEY_SIZE]);

impl Seed {
    pub fn new() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed(bytes)
    }

    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        Ok(secp256k1::SecretKey::from_slice(&self.0)?)
    }

    pub fn into_inner(self) -> [u8; SECRET_KEY_SIZE] {
        self.0
    }
}

impl Default for Seed {
    fn default() -> Self {
        Self::new()
    }
}

impl From<[u8; SECRET_KEY_SIZE]> for Seed {
    fn from(from: [u8; SECRET_KEY_SIZE]) -> Self {
        Self(from)
    }
}
