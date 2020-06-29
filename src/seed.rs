use ::bitcoin::secp256k1;
use bip39::{self, Language, Mnemonic};
use rand::prelude::*;
use sha3::{Digest, Sha3_256};

/// The seed is used as a the root seed for both Ethereum and Bitcoin HD Wallets.
/// API is also provided to allow the backup of the seed via a BIP39 24-words seed phrase
/// (or mnemonic). Do note that that BIP-39/BIP-32/BIP-44 do not specify how the mnemonic
/// seed should be used to make the extended/master/root private key.
/// Here, we simply hash the mnemonic seed once with SHA3-256.
#[derive(Debug, Clone, Copy)]
pub struct Seed {
    entropy: [u8; ENTROPY_SIZE],
}

/// 32 bytes entropy => 24 seed words
const ENTROPY_SIZE: usize = 32;

impl Seed {
    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        let bytes = &self.bytes();

        // TODO: pass it through hmac to get different key for Ethereum and Bitcoin?
        // See https://github.com/trezor/python-mnemonic/blob/709c52e99ae05cdaf512a3d0e2847451d682820a/mnemonic/mnemonic.py#L251
        // Or, look at the mnemonic use of other Bitcoin wallets?

        let mut hasher = Sha3_256::new();
        hasher.update(bytes);
        let hash = hasher.finalize();

        Ok(secp256k1::SecretKey::from_slice(&hash)?)
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
        let mut bytes = [0u8; ENTROPY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed { entropy: bytes }
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

    #[test]
    fn generated_seed_is_twenty_four_words_seed_phrase() {
        let seed = Seed::default();
        let phrase = seed.phrase();
        let words: Vec<_> = phrase.split(' ').collect();
        assert_eq!(words.len(), 24)
    }
}
