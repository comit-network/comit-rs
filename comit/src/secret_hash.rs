use crate::Secret;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

const LENGTH: usize = 32;

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
#[error("invalid length, expected: {expected:?}, got: {got:?}")]
pub struct InvalidLength {
    expected: usize,
    got: usize,
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SecretHash([u8; LENGTH]);

impl SecretHash {
    pub fn new(secret: Secret) -> Self {
        let hash = Sha256::digest(secret.as_raw_secret()).into();

        SecretHash(hash)
    }

    pub fn from_vec(vec: &[u8]) -> Result<Self, InvalidLength> {
        if vec.len() != LENGTH {
            return Err(InvalidLength {
                expected: LENGTH,
                got: vec.len(),
            });
        }
        let mut data = [0; LENGTH];
        let vec = &vec[..LENGTH];
        data.copy_from_slice(vec);

        Ok(SecretHash(data))
    }

    pub fn as_raw(&self) -> &[u8; LENGTH] {
        &self.0
    }

    pub fn into_raw(self) -> [u8; LENGTH] {
        self.0
    }
}

impl fmt::Debug for SecretHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(&format!("SecretHash({:x})", self))
    }
}

impl<'a> From<&'a SecretHash> for SecretHash {
    fn from(s: &'a SecretHash) -> Self {
        *s
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

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
pub enum FromStrError {
    #[error("failed to decode bytes as hex")]
    HexDecode(#[from] hex::FromHexError),
    #[error("failed to construct secret hash from bytes")]
    FromVec(#[from] InvalidLength),
}

impl FromStr for SecretHash {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let vec = hex::decode(s)?;
        let secret_hash = Self::from_vec(&vec)?;

        Ok(secret_hash)
    }
}

impl From<[u8; LENGTH]> for SecretHash {
    fn from(hash: [u8; LENGTH]) -> Self {
        SecretHash(hash)
    }
}

impl From<SecretHash> for [u8; 32] {
    fn from(secret_hash: SecretHash) -> [u8; 32] {
        secret_hash.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Secret;

    #[test]
    fn new_secret_hash_as_hex() {
        let bytes = b"hello world, you are beautiful!!";
        let secret = Secret::from(*bytes);
        assert_eq!(
            SecretHash::new(secret).to_string(),
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        );
    }

    #[test]
    fn secret_hash_should_be_displayed_as_hex() {
        let bytes = b"hello world, you are beautiful!!";
        let secret = Secret::from(*bytes);

        let hash = SecretHash::new(secret);

        let formatted_hash = hash.to_string();

        assert_eq!(
            formatted_hash,
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        )
    }
}
