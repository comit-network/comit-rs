use ::bitcoin::secp256k1;
use ::bitcoin::secp256k1::constants::SECRET_KEY_SIZE;
use rand::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Seed([u8; SECRET_KEY_SIZE]);

impl Seed {
    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        Ok(secp256k1::SecretKey::from_slice(&self.0)?)
    }
}

impl Default for Seed {
    fn default() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let _seed = Seed::default();
    }
}
