use secp256k1::{self, rand::Rng, Message, PublicKey, RecoverableSignature, Secp256k1, Signature};
use std::str::FromStr;

#[derive(Debug)]
pub enum Error {
    NoSecretKeyProvided,
    Secp256k1(secp256k1::Error),
}

impl From<secp256k1::Error> for Error {
    fn from(err: secp256k1::Error) -> Self {
        Error::Secp256k1(err)
    }
}

#[derive(Debug)]
pub struct Builder {
    secp: Secp256k1<secp256k1::All>,
    secret_key: Option<secp256k1::SecretKey>,
}

impl Builder {
    pub fn new(secp: Secp256k1<secp256k1::All>) -> Builder {
        Builder {
            secp,
            secret_key: None,
        }
    }

    pub fn secret_key(self, secret_key: secp256k1::SecretKey) -> Builder {
        Builder {
            secret_key: Some(secret_key),
            ..self
        }
    }

    pub fn rng<R: Rng>(self, rng: &mut R) -> Builder {
        Builder {
            secret_key: Some(secp256k1::SecretKey::new(rng)),
            ..self
        }
    }

    pub fn secret_key_slice(self, data: &[u8]) -> Result<Builder, Error> {
        Ok(
            secp256k1::SecretKey::from_slice(data).map(|secret_key| Builder {
                secret_key: Some(secret_key),
                ..self
            })?,
        )
    }

    pub fn secret_key_hex(self, hex: &str) -> Result<Builder, Error> {
        Ok(
            secp256k1::SecretKey::from_str(hex).map(|secret_key| Builder {
                secret_key: Some(secret_key),
                ..self
            })?,
        )
    }

    pub fn build(self) -> Result<SecretKey, Error> {
        match self {
            Builder {
                secret_key: None, ..
            } => Err(Error::NoSecretKeyProvided),
            Builder {
                secret_key: Some(secret_key),
                secp,
            } => Ok(SecretKey { secret_key, secp }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretKey {
    secret_key: secp256k1::SecretKey,
    secp: Secp256k1<secp256k1::All>,
}

impl SecretKey {
    pub fn public_key(&self) -> PublicKey {
        secp256k1::PublicKey::from_secret_key(&self.secp, &self.secret_key)
    }

    pub fn sign_ecdsa(&self, message: Message) -> Signature {
        self.secp.sign(&message, &self.secret_key)
    }

    pub fn sign_ecdsa_recoverable(&self, message: Message) -> RecoverableSignature {
        self.secp.sign_recoverable(&message, &self.secret_key)
    }
}

impl From<SecretKey> for secp256k1::SecretKey {
    fn from(secret_key: SecretKey) -> Self {
        secret_key.secret_key
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn correct_keypair_from_secret_key_slice() {
        // taken from: https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
        let keypair = Builder::new(Secp256k1::new())
            .secret_key_slice(
                &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                    .unwrap(),
            )
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            keypair.public_key(),
            PublicKey::from_str(
                "0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352"
            )
            .unwrap(),
        )
    }
}
