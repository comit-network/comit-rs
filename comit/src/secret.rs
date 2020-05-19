use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

const LENGTH: usize = 32;

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
#[error("invalid length, expected: {expected:?}, got: {got:?}")]
pub struct InvalidLength {
    expected: usize,
    got: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Secret([u8; LENGTH]);

impl From<[u8; LENGTH]> for Secret {
    fn from(secret: [u8; LENGTH]) -> Self {
        Secret(secret)
    }
}

impl Secret {
    pub fn from_vec(vec: &[u8]) -> Result<Secret, InvalidLength> {
        if vec.len() != LENGTH {
            return Err(InvalidLength {
                expected: LENGTH,
                got: vec.len(),
            });
        }
        let mut data = [0; LENGTH];
        let vec = &vec[..LENGTH];
        data.copy_from_slice(vec);
        Ok(Secret(data))
    }

    pub fn as_raw_secret(&self) -> &[u8; LENGTH] {
        &self.0
    }

    pub fn into_raw_secret(self) -> [u8; LENGTH] {
        self.0
    }
}

impl fmt::LowerHex for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(hex::encode(&self.0).as_str())
    }
}

#[derive(PartialEq, Clone, Copy, Debug, thiserror::Error)]
pub enum FromStrError {
    #[error("failed to decode bytes as hex")]
    HexDecode(#[from] hex::FromHexError),
    #[error("failed to construct secret from bytes")]
    FromVec(#[from] InvalidLength),
}

impl FromStr for Secret {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let vec = hex::decode(s)?;
        let secret = Self::from_vec(&vec)?;

        Ok(secret)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = Secret;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_str("a hex encoded 32 byte value")
            }

            fn visit_str<E>(self, v: &str) -> Result<Secret, E>
            where
                E: de::Error,
            {
                Secret::from_str(v).map_err(|_| {
                    de::Error::invalid_value(de::Unexpected::Str(v), &"hex encoded bytes")
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:x}", self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_length_from_str() {
        let result =
            Secret::from_str("68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4c");

        assert!(result.is_err());
    }
}
