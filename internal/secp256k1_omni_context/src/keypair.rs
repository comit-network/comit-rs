use secp256k1::{
    self, rand::Rng, Error, Message, PublicKey, RecoverableSignature, Secp256k1, SecretKey,
    Signature,
};
use std::{convert::Into, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPair {
    secret_key: SecretKey,
    public_key: PublicKey,
    secp: Secp256k1<secp256k1::All>,
}

// TODO: Make it a builder instead.
impl KeyPair {
    pub fn new<R: Rng>(secp: Secp256k1<secp256k1::All>, rng: &mut R) -> KeyPair {
        (secp, SecretKey::new(rng)).into()
    }

    // TODO: Should this really consume self?
    pub fn secret_key(self) -> SecretKey {
        self.secret_key
    }

    pub fn public_key(self) -> PublicKey {
        self.public_key
    }

    pub fn keys(self) -> (SecretKey, PublicKey) {
        (self.secret_key, self.public_key)
    }

    pub fn from_secret_key_slice(
        secp: Secp256k1<secp256k1::All>,
        data: &[u8],
    ) -> Result<KeyPair, Error> {
        SecretKey::from_slice(data).map(|secret_key| (secp, secret_key).into())
    }

    pub fn from_secret_key_hex(
        secp_context: Secp256k1<secp256k1::All>,
        key: &str,
    ) -> Result<KeyPair, Error> {
        SecretKey::from_str(key).map(|secret_key| (secp_context, secret_key).into())
    }

    pub fn sign_ecdsa(&self, message: Message) -> Signature {
        self.secp.sign(&message, &self.secret_key)
    }

    pub fn sign_ecdsa_recoverable(&self, message: Message) -> RecoverableSignature {
        self.secp.sign_recoverable(&message, &self.secret_key)
    }
}

impl From<(Secp256k1<secp256k1::All>, SecretKey)> for KeyPair {
    fn from(secp_secret_key: (Secp256k1<secp256k1::All>, SecretKey)) -> KeyPair {
        let (secp, secret_key) = secp_secret_key;
        KeyPair {
            public_key: secp256k1::PublicKey::from_secret_key(&secp, &secret_key),
            secret_key,
            secp,
        }
    }
}

impl From<(Secp256k1<secp256k1::All>, SecretKey, PublicKey)> for KeyPair {
    fn from(pair: (Secp256k1<secp256k1::All>, SecretKey, PublicKey)) -> KeyPair {
        KeyPair {
            secp: pair.0,
            secret_key: pair.1,
            public_key: pair.2,
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

    #[test]
    fn correct_keypair_from_secret_key_slice() {
        let secp: Secp256k1<secp256k1::All> = Secp256k1::new();
        // taken from: https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
        let keypair = KeyPair::from_secret_key_slice(
            secp,
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap(),
        )
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
