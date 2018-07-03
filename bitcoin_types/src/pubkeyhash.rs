use bitcoin::util::address::Address;
use bitcoin::util::address::Payload::WitnessProgram;
use bitcoin::util::hash::Hash160;
use secp256k1::{PublicKey, SecretKey};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct PubkeyHash(Hash160);

impl From<Hash160> for PubkeyHash {
    fn from(hash: Hash160) -> PubkeyHash {
        PubkeyHash(hash)
    }
}

impl From<Address> for PubkeyHash {
    fn from(address: Address) -> PubkeyHash {
        match address.payload {
            WitnessProgram(witness) => PubkeyHash(Hash160::from(witness.program())),
            // TODO: from/into should never fail. Remove this panic by
            // creating a PubkeyAddress type which is guaranteed to
            // have a PubkeyHash inside it.
            _ => panic!("Address {} isn't a pubkey address", address.to_string()),
        }
    }
}

impl From<PublicKey> for PubkeyHash {
    fn from(public_key: PublicKey) -> PubkeyHash {
        PubkeyHash(Hash160::from_data(&public_key.serialize()))
    }
}

impl From<SecretKey> for PubkeyHash {
    fn from(secret_key: SecretKey) -> PubkeyHash {
        PublicKey::from_secret_key(&*super::SECP, &secret_key)
            .unwrap()
            .into()
    }
}

impl<'a> From<&'a [u8]> for PubkeyHash {
    fn from(data: &'a [u8]) -> PubkeyHash {
        PubkeyHash(Hash160::from(data))
    }
}

impl AsRef<[u8]> for PubkeyHash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl Into<Hash160> for PubkeyHash {
    fn into(self) -> Hash160 {
        self.0
    }
}

impl fmt::LowerHex for PubkeyHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(format!("{:?}", self.0).as_str())
    }
}

#[cfg(test)]
mod test {
    extern crate hex;
    use super::*;
    use std::str::FromStr;

    #[test]
    fn given_an_bitcoin_address_return_pubkey_hash() {
        let address = Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap();
        let pubkey_hash: PubkeyHash = address.into();

        assert_eq!(
            format!("{:x}", pubkey_hash),
            "c021f17be99c6adfbcba5d38ee0d292c0399d2f5"
        );
    }

    #[test]
    fn correct_pubkey_hash_from_private_key() {
        // taken from https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
        let secret_key = SecretKey::from_slice(
            &*super::super::SECP,
            &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                .unwrap()[..],
        ).unwrap();

        let pubkey_hash: PubkeyHash = secret_key.into();

        assert_eq!(
            pubkey_hash,
            PubkeyHash::from(&hex::decode("f54a5851e9372b87810a8e60cdd2e7cfd80b6e31").unwrap()[..])
        )
    }

}
