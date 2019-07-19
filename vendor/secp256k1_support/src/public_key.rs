use secp256k1;
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PublicKey(secp256k1::PublicKey);

impl PublicKey {
    pub fn inner(&self) -> &secp256k1::PublicKey {
        &self.0
    }
}

impl From<secp256k1::PublicKey> for PublicKey {
    fn from(pubkey: secp256k1::PublicKey) -> Self {
        PublicKey(pubkey)
    }
}

impl From<PublicKey> for secp256k1::PublicKey {
    fn from(public_key: PublicKey) -> secp256k1::PublicKey {
        public_key.0
    }
}

impl FromStr for PublicKey {
    type Err = secp256k1::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        secp256k1::PublicKey::from_str(s).map(PublicKey)
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("A hex-encoded compressed SECP256k1 public key")
            }

            fn visit_str<E>(self, hex_pubkey: &str) -> Result<PublicKey, E>
            where
                E: de::Error,
            {
                secp256k1::PublicKey::from_str(hex_pubkey)
                    .map(PublicKey)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self.0))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn pubkey_from_hex() {
        let pubkey = PublicKey(
            secp256k1::PublicKey::from_slice(&[
                3, 23, 183, 225, 206, 31, 159, 148, 195, 42, 67, 115, 146, 41, 248, 140, 11, 3, 51,
                41, 111, 180, 110, 143, 114, 134, 88, 73, 198, 174, 52, 184, 78,
            ])
            .unwrap(),
        );

        let from_hex = PublicKey::from_str(
            "0317b7e1ce1f9f94c32a43739229f88c0b0333296fb46e8f72865849c6ae34b84e",
        )
        .unwrap();

        assert_eq!(pubkey, from_hex);
    }

    #[test]
    fn serialize_to_hex() {
        let pubkey = PublicKey(
            secp256k1::PublicKey::from_slice(&[
                3, 23, 183, 225, 206, 31, 159, 148, 195, 42, 67, 115, 146, 41, 248, 140, 11, 3, 51,
                41, 111, 180, 110, 143, 114, 134, 88, 73, 198, 174, 52, 184, 78,
            ])
            .unwrap(),
        );

        let string = serde_json::to_string(&pubkey).unwrap();

        assert_eq!(
            string,
            r#""0317b7e1ce1f9f94c32a43739229f88c0b0333296fb46e8f72865849c6ae34b84e""#
        )
    }
}
