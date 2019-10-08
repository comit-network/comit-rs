use crate::Hash160;
use bitcoin::hashes::Hash;
use hex::{self, FromHex};
use secp256k1_keypair::{KeyPair, PublicKey};
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{
    convert::TryFrom,
    fmt::{self, Display},
};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PubkeyHash(Hash160);

impl From<Hash160> for PubkeyHash {
    fn from(hash: Hash160) -> PubkeyHash {
        PubkeyHash(hash)
    }
}

impl From<PublicKey> for PubkeyHash {
    fn from(public_key: PublicKey) -> PubkeyHash {
        PubkeyHash(
            <bitcoin::hashes::hash160::Hash as bitcoin::hashes::Hash>::hash(
                &public_key.serialize(),
            ),
        )
    }
}

impl From<KeyPair> for PubkeyHash {
    fn from(key_pair: KeyPair) -> Self {
        key_pair.public_key().into()
    }
}

impl<'a> TryFrom<&'a [u8]> for PubkeyHash {
    type Error = bitcoin::hashes::error::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(PubkeyHash(Hash160::from_slice(value)?))
    }
}

#[derive(Debug)]
pub enum FromHexError {
    HexConversion(hex::FromHexError),
    HashConversion(bitcoin::hashes::error::Error),
}

impl From<hex::FromHexError> for FromHexError {
    fn from(err: hex::FromHexError) -> Self {
        FromHexError::HexConversion(err)
    }
}

impl From<bitcoin::hashes::error::Error> for FromHexError {
    fn from(err: bitcoin::hashes::error::Error) -> Self {
        FromHexError::HashConversion(err)
    }
}

impl Display for FromHexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:?}", &self)
    }
}

impl FromHex for PubkeyHash {
    type Error = FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        Ok(PubkeyHash::try_from(hex::decode(hex)?.as_ref())?)
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(format!("{:?}", self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for PubkeyHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = PubkeyHash;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("A hex-encoded compressed SECP256k1 public key")
            }

            fn visit_str<E>(self, hex_pubkey: &str) -> Result<PubkeyHash, E>
            where
                E: de::Error,
            {
                PubkeyHash::from_hex(hex_pubkey).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for PubkeyHash {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(hex::encode(self.0.into_inner()).as_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::PrivateKey;
    use secp256k1_keypair::KeyPair;
    use std::str::FromStr;

    #[test]
    fn correct_pubkeyhash_from_private_key() {
        let private_key =
            PrivateKey::from_str("L253jooDhCtNXJ7nVKy7ijtns7vU4nY49bYWqUH8R9qUAUZt87of").unwrap();
        let keypair: KeyPair = private_key.key.into();
        let pubkey_hash: PubkeyHash = keypair.public_key().into();

        assert_eq!(
            pubkey_hash,
            PubkeyHash::try_from(
                &hex::decode("8bc513e458372a3b3bb05818d09550295ce15949").unwrap()[..]
            )
            .unwrap()
        )
    }

    #[test]
    fn roudtrip_serialization_of_pubkeyhash() {
        let public_key = PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let pubkey_hash: PubkeyHash = public_key.into();
        let serialized = serde_json::to_string(&pubkey_hash).unwrap();
        assert_eq!(serialized, "\"ac2db2f2615c81b83fe9366450799b4992931575\"");
        let deserialized = serde_json::from_str::<PubkeyHash>(serialized.as_str()).unwrap();
        assert_eq!(deserialized, pubkey_hash);
    }
}
