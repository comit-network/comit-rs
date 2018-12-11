use crate::public_key::PublicKey;
use hex;
use rand::Rng;
use secp256k1::{self, Message, RecoverableSignature, SecretKey, Signature};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct KeyPair {
    secret_key: SecretKey,
    public_key: PublicKey,
}

#[derive(Debug)]
pub enum Error {
    Secp256k1(secp256k1::Error),
    Hex(hex::FromHexError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Secp256k1(_) => write!(f, "Not a secp256k1 private key"),
            Error::Hex(_) => write!(f, "Invalid hex value"),
        }
    }
}

impl KeyPair {
    pub fn new<R: Rng>(rng: &mut R) -> KeyPair {
        SecretKey::new(&*super::SECP, rng).into()
    }

    pub fn secret_key(self) -> SecretKey {
        self.secret_key
    }

    pub fn public_key(self) -> PublicKey {
        self.public_key
    }

    pub fn from_secret_key_slice(data: &[u8]) -> Result<KeyPair, Error> {
        SecretKey::from_slice(&*super::SECP, data)
            .map(|secret_key| secret_key.into())
            .map_err(Error::Secp256k1)
    }

    pub fn from_secret_key_hex(key: &str) -> Result<KeyPair, Error> {
        let bytes = hex::decode(key).map_err(Error::Hex)?;
        KeyPair::from_secret_key_slice(&bytes[..])
    }

    pub fn sign_ecdsa(&self, message: Message) -> Signature {
        super::SECP.sign(&message, &self.secret_key)
    }

    pub fn sign_ecdsa_recoverable(&self, message: Message) -> RecoverableSignature {
        super::SECP.sign_recoverable(&message, &self.secret_key)
    }
}

impl From<SecretKey> for KeyPair {
    fn from(secret_key: SecretKey) -> KeyPair {
        KeyPair {
            public_key: secp256k1::PublicKey::from_secret_key(&*super::SECP, &secret_key).into(),
            secret_key,
        }
    }
}

impl From<(SecretKey, PublicKey)> for KeyPair {
    fn from(pair: (SecretKey, PublicKey)) -> KeyPair {
        KeyPair {
            secret_key: pair.0,
            public_key: pair.1,
        }
    }
}

impl From<KeyPair> for (SecretKey, PublicKey) {
    fn from(keypair: KeyPair) -> (SecretKey, PublicKey) {
        (keypair.secret_key, keypair.public_key)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hex;
    use hex::FromHex;

    #[test]
    fn correct_keypair_from_secret_key_slice() {
        // taken from: https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
        let keypair = KeyPair::from_secret_key_slice(
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            keypair.public_key(),
            PublicKey::from_hex(
                "0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352"
            )
            .unwrap(),
        )
    }
}
