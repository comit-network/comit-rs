extern crate secp256k1;
#[macro_use]
extern crate lazy_static;

use secp256k1::Secp256k1;
pub use secp256k1::{PublicKey, SecretKey};

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

pub trait ToPublicKey {
    fn to_public_key(&self) -> PublicKey;
}

impl ToPublicKey for SecretKey {
    fn to_public_key(&self) -> PublicKey {
        PublicKey::from_secret_key(&*SECP, &self).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    extern crate hex;

    #[test]
    fn correct_public_key_from_private_key() {
        // taken from: https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
        let secret_key = SecretKey::from_slice(
            &*SECP,
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap()[..],
        ).unwrap();

        let public_key = secret_key.to_public_key();

        assert_eq!(
            public_key,
            PublicKey::from_slice(
                &*SECP,
                &hex::decode("0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352")
                    .unwrap()[..]
            ).unwrap()
        )
    }
}
