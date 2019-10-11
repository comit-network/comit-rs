use hex::{self, FromHex};
use rust_bitcoin::{
    hashes::{hash160, Hash},
    secp256k1::{self, PublicKey, Secp256k1, SecretKey},
};
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{
    convert::TryFrom,
    fmt::{self, Display},
};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PubkeyHash(hash160::Hash);

#[allow(dead_code)] // Only used in tests at the moment
impl PubkeyHash {
    fn new<C: secp256k1::Signing>(secp: &Secp256k1<C>, secret_key: &SecretKey) -> Self {
        secp256k1::PublicKey::from_secret_key(secp, secret_key).into()
    }
}

impl From<hash160::Hash> for PubkeyHash {
    fn from(hash: hash160::Hash) -> PubkeyHash {
        PubkeyHash(hash)
    }
}

impl From<PublicKey> for PubkeyHash {
    fn from(public_key: PublicKey) -> PubkeyHash {
        PubkeyHash(
            <rust_bitcoin::hashes::hash160::Hash as rust_bitcoin::hashes::Hash>::hash(
                &public_key.serialize(),
            ),
        )
    }
}

impl<'a> TryFrom<&'a [u8]> for PubkeyHash {
    type Error = rust_bitcoin::hashes::error::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(PubkeyHash(hash160::Hash::from_slice(value)?))
    }
}

#[derive(Debug)]
pub enum FromHexError {
    HexConversion(hex::FromHexError),
    HashConversion(rust_bitcoin::hashes::error::Error),
}

impl From<hex::FromHexError> for FromHexError {
    fn from(err: hex::FromHexError) -> Self {
        FromHexError::HexConversion(err)
    }
}

impl From<rust_bitcoin::hashes::error::Error> for FromHexError {
    fn from(err: rust_bitcoin::hashes::error::Error) -> Self {
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

impl Into<hash160::Hash> for PubkeyHash {
    fn into(self) -> hash160::Hash {
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
    use rust_bitcoin::PrivateKey;
    use std::str::FromStr;

    #[test]
    fn correct_pubkeyhash_from_private_key() {
        let secp = Secp256k1::signing_only();

        let private_key =
            PrivateKey::from_str("L253jooDhCtNXJ7nVKy7ijtns7vU4nY49bYWqUH8R9qUAUZt87of").unwrap();
        let pubkey_hash = PubkeyHash::new(&secp, &private_key.key);

        assert_eq!(
            pubkey_hash,
            PubkeyHash::try_from(
                &hex::decode("8bc513e458372a3b3bb05818d09550295ce15949").unwrap()[..]
            )
            .unwrap()
        )
    }

    #[test]
    fn roundtrip_serialization_of_pubkeyhash() {
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
