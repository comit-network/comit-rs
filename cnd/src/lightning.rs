//! This module is the home of lightning-specific types and functionality that
//! is needed across several places in cnd.
//!
//! This involves:
//!     - Creating wrapper types to allow for more ergonomic APIs of upstream
//!       libraries
//!     - Common functionality that is not (yet) available upstream

use bitcoin::secp256k1;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, str::FromStr};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey(bitcoin::PublicKey);

impl PublicKey {
    pub fn from_secret_key<C>(
        secp: &secp256k1::Secp256k1<C>,
        secret_key: &secp256k1::SecretKey,
    ) -> Self
    where
        C: secp256k1::Signing,
    {
        secp256k1::PublicKey::from_secret_key(secp, secret_key).into()
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

impl From<secp256k1::PublicKey> for PublicKey {
    fn from(key: secp256k1::PublicKey) -> Self {
        Self(bitcoin::PublicKey {
            compressed: true, // we always want the PublicKey to be serialized in a compressed way
            key,
        })
    }
}

impl From<PublicKey> for bitcoin::PublicKey {
    fn from(pubkey: PublicKey) -> bitcoin::PublicKey {
        pubkey.0
    }
}

impl From<bitcoin::util::key::PublicKey> for PublicKey {
    fn from(key: bitcoin::util::key::PublicKey) -> Self {
        Self(key)
    }
}

impl FromStr for PublicKey {
    type Err = bitcoin::util::key::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bitcoin::PublicKey::from_str(s)?.into())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeyVisitor;

        impl<'de> Visitor<'de> for PublicKeyVisitor {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a hex-encoded, compressed public key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(PublicKeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn serialize_bitcoin_identity() {
        let secp_pubkey = secp256k1::PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let pubkey = PublicKey::from(secp_pubkey);

        let str = serde_json::to_string(&pubkey).unwrap();
        assert_eq!(
            str,
            "\"02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275\""
        )
    }

    #[test]
    fn deserialize_bitcoin_identity() {
        let pubkey_str = "\"02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275\"";
        let pubkey = serde_json::from_str::<PublicKey>(pubkey_str).unwrap();

        let expected_pubkey = secp256k1::PublicKey::from_str(
            "02c2a8efce029526d364c2cf39d89e3cdda05e5df7b2cbfc098b4e3d02b70b5275",
        )
        .unwrap();
        let expected_pubkey = PublicKey::from(expected_pubkey);

        assert_eq!(pubkey, expected_pubkey);
    }
}
