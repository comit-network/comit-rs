use ::bitcoin::secp256k1;
use ::bitcoin::secp256k1::constants::SECRET_KEY_SIZE;
use bip39::{self, Language, Mnemonic};
use rand::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Seed {
    entropy: [u8; SECRET_KEY_SIZE],
}

impl Seed {
    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        Ok(secp256k1::SecretKey::from_slice(&self.bytes())?)
    }

    pub fn bytes(&self) -> Vec<u8> {
        // TODO: Allow usage of password
        bip39::Seed::new(&self.mnemonic(), "").as_bytes().to_vec()
    }

    pub fn phrase(&self) -> String {
        self.mnemonic().phrase().to_owned()
    }

    fn mnemonic(&self) -> Mnemonic {
        Mnemonic::from_entropy(&self.entropy, Language::English).expect("entropy size is supported")
    }
}

impl Default for Seed {
    fn default() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed { entropy: bytes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let _seed = Seed::default();
    }

    #[test]
    fn generated_seed_is_twenty_four_words_seed_phrase() {
        let seed = Seed::default();
        let phrase = seed.phrase();
        let words: Vec<_> = phrase.split(' ').collect();
        assert_eq!(words.len(), 24)
    }
}
