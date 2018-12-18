use crypto::{digest::Digest, sha2::Sha256};
use rand::{Rng, ThreadRng};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

#[derive(PartialEq, Debug)]
pub enum FromErr {
    InvalidLength { expected: usize, got: usize },
    FromHex(hex::FromHexError),
}

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SecretHash([u8; Self::LENGTH]);

impl SecretHash {
    pub const LENGTH: usize = 32;

    pub fn from_vec(vec: &[u8]) -> Result<SecretHash, FromErr> {
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
        s.clone()
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

impl FromStr for SecretHash {
    type Err = FromErr;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let vec = hex::decode(s)?;
        Self::from_vec(&vec)
    }
}

impl From<[u8; Self::LENGTH]> for SecretHash {
    fn from(hash: [u8; Self::LENGTH]) -> Self {
        SecretHash(hash)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Secret([u8; Self::LENGTH]);

impl From<[u8; Self::LENGTH]> for Secret {
    fn from(secret: [u8; Self::LENGTH]) -> Self {
        Secret(secret)
    }
}

impl From<Secret> for SecretHash {
    fn from(secret: Secret) -> Self {
        secret.hash()
    }
}

impl Secret {
    // Both values need to stay the same!
    pub const LENGTH: usize = 32;
    pub const LENGTH_U8: u8 = 32;

    pub fn generate<T: RandomnessSource>(rng: &mut T) -> Secret {
        let random_bytes = rng.gen_random_bytes(Self::LENGTH);
        let mut secret = [0; Self::LENGTH];
        secret.copy_from_slice(&random_bytes[..]);
        Secret::from(secret)
    }

    pub fn from_vec(vec: &[u8]) -> Result<Secret, FromErr> {
        if vec.len() != Self::LENGTH {
            return Err(FromErr::InvalidLength {
                expected: Self::LENGTH,
                got: vec.len(),
            });
        }
        let mut data = [0; Self::LENGTH];
        let vec = &vec[..Self::LENGTH];
        data.copy_from_slice(vec);
        Ok(Secret(data))
    }

    pub fn hash(&self) -> SecretHash {
        let mut sha = Sha256::new();
        sha.input(&self.0);

        let mut result: [u8; SecretHash::LENGTH] = [0; SecretHash::LENGTH];
        sha.result(&mut result);
        SecretHash::from(result)
    }

    pub fn raw_secret(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}

impl fmt::LowerHex for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(hex::encode(&self.0).as_str())
    }
}

impl From<hex::FromHexError> for FromErr {
    fn from(err: hex::FromHexError) -> Self {
        FromErr::FromHex(err)
    }
}

impl FromStr for Secret {
    type Err = FromErr;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let vec = hex::decode(s)?;
        Self::from_vec(&vec)
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

pub trait RandomnessSource {
    fn gen_random_bytes(&mut self, nbytes: usize) -> Vec<u8>;
}

impl RandomnessSource for ThreadRng {
    fn gen_random_bytes(&mut self, nbytes: usize) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![0; nbytes];
        self.fill_bytes(&mut buf);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn gen_random_bytes_not_zeros() {
        let mut rng = rand::thread_rng();

        let empty_buf: Vec<u8> = vec![0; 32];
        let buf = rng.gen_random_bytes(32);
        assert_eq!(buf.len(), 32);
        assert_ne!(buf, empty_buf);
    }

    #[test]
    fn new_secret_hash_as_hex() {
        let bytes = b"hello world, you are beautiful!!";
        let secret = Secret::from(*bytes);
        assert_eq!(
            secret.hash().to_string(),
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        );
    }

    #[test]
    fn secret_hash_should_be_displayed_as_hex() {
        let bytes = b"hello world, you are beautiful!!";
        let secret = Secret::from(*bytes);

        let hash = secret.hash();

        let formatted_hash = format!("{}", hash);

        assert_eq!(
            formatted_hash,
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        )
    }

    #[test]
    fn round_trip_secret_serialization() {
        let mut rng = rand::thread_rng();

        let secret = Secret::generate(&mut rng);

        let json_secret = serde_json::to_string(&secret).unwrap();
        let deser_secret = serde_json::from_str::<Secret>(json_secret.as_str()).unwrap();

        assert_eq!(deser_secret, secret);
    }

    #[test]
    fn invalid_length_from_str() {
        let result =
            Secret::from_str("68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4c");

        assert!(result.is_err());

        assert_eq!(
            result.unwrap_err(),
            FromErr::InvalidLength {
                expected: 32,
                got: 31
            }
        );
    }

    #[test]
    fn secret_length_is_consistent() {
        assert_eq!(Secret::LENGTH, usize::from(Secret::LENGTH_U8));
    }
}
