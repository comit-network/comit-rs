use crate::rfc003::secret::Secret;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SecretHash([u8; Self::LENGTH]);

impl SecretHash {
    pub const LENGTH: usize = 32;

    pub fn raw(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}

impl Debug for SecretHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(&format!("SecretHash({:x})", self))
    }
}

impl<'a> From<&'a SecretHash> for SecretHash {
    fn from(s: &'a SecretHash) -> Self {
        *s
    }
}

impl From<Secret> for SecretHash {
    fn from(secret: Secret) -> Self {
        secret.hash()
    }
}

impl fmt::Display for SecretHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(&format!("{:x}", self))
    }
}

impl fmt::LowerHex for SecretHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(hex::encode(&self.0).as_str())
    }
}

impl Serialize for SecretHash {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:x}", self))
    }
}

impl<'de> Deserialize<'de> for SecretHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = SecretHash;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("a hex encoded 32 byte value")
            }

            fn visit_str<E>(self, v: &str) -> Result<SecretHash, E>
            where
                E: de::Error,
            {
                SecretHash::from_str(v).map_err(|_| {
                    de::Error::invalid_value(de::Unexpected::Str(v), &"hex encoded bytes")
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(PartialEq, Debug)]
pub enum FromErr {
    InvalidLength { expected: usize, got: usize },
    FromHex(hex::FromHexError),
}

impl From<hex::FromHexError> for FromErr {
    fn from(err: hex::FromHexError) -> Self {
        FromErr::FromHex(err)
    }
}

impl FromStr for SecretHash {
    type Err = FromErr;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let vec = hex::decode(s)?;
        if vec.len() != Self::LENGTH {
            return Err(FromErr::InvalidLength {
                expected: Self::LENGTH,
                got: vec.len(),
            });
        }
        let mut data = [0; Self::LENGTH];
        let vec = &vec[..Self::LENGTH];
        data.copy_from_slice(vec);
        Ok(SecretHash(data))
    }
}

impl From<[u8; Self::LENGTH]> for SecretHash {
    fn from(hash: [u8; Self::LENGTH]) -> Self {
        SecretHash(hash)
    }
}
