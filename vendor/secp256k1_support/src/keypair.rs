use rand::Rng;
use secp256k1::{Error, Message, PublicKey, RecoverableSignature, SecretKey, Signature};

#[derive(Clone, Debug, PartialEq)]
pub struct KeyPair {
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl KeyPair {
    pub fn new<R: Rng>(rng: &mut R) -> KeyPair {
        SecretKey::new(&*super::SECP, rng).into()
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn from_secret_key_slice(data: &[u8]) -> Result<KeyPair, Error> {
        SecretKey::from_slice(&*super::SECP, data).map(|secret_key| secret_key.into())
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn sign_ecdsa(&self, message: Message) -> Signature {
        super::SECP.sign(&message, &self.secret_key).unwrap()
    }

    pub fn sign_ecdsa_recoverable(&self, message: Message) -> RecoverableSignature {
        super::SECP
            .sign_recoverable(&message, &self.secret_key)
            .unwrap()
    }
}

impl From<SecretKey> for KeyPair {
    fn from(secret_key: SecretKey) -> KeyPair {
        KeyPair {
            public_key: PublicKey::from_secret_key(&*super::SECP, &secret_key).unwrap(),
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

impl Into<(SecretKey, PublicKey)> for KeyPair {
    fn into(self) -> (SecretKey, PublicKey) {
        (self.secret_key, self.public_key)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    extern crate hex;

    #[test]
    fn correct_keypair_from_secret_key_slice() {
        // taken from: https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
        let keypair = KeyPair::from_secret_key_slice(&hex::decode(
            "18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725",
        ).unwrap())
            .unwrap();

        assert_eq!(
            *keypair.public_key(),
            PublicKey::from_slice(
                &*super::super::SECP,
                &hex::decode("0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352")
                    .unwrap()[..]
            ).unwrap()
        )
    }
}
