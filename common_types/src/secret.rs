use crypto::digest::Digest;
use crypto::sha2::Sha256;
use hex;
use rand::{OsRng, Rng};
use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const SHA256_DIGEST_LENGTH: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct SecretHash(pub Vec<u8>);

impl<'a> From<&'a SecretHash> for SecretHash {
    fn from(s: &'a SecretHash) -> Self {
        s.clone()
    }
}

impl fmt::Display for SecretHash {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&format!("{:x}", self))
    }
}

impl fmt::LowerHex for SecretHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
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

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
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
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        hex::decode(s).map(SecretHash)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Secret([u8; SHA256_DIGEST_LENGTH]);

impl From<[u8; SHA256_DIGEST_LENGTH]> for Secret {
    fn from(secret: [u8; SHA256_DIGEST_LENGTH]) -> Self {
        Secret(secret)
    }
}

impl Secret {
    pub fn generate<T: RandomnessSource>(rng: &mut T) -> Secret {
        let random_bytes = rng.gen_random_bytes(SHA256_DIGEST_LENGTH);
        let mut secret = [0; SHA256_DIGEST_LENGTH];
        secret.copy_from_slice(&random_bytes[..]);
        Secret::from(secret)
    }

    pub fn hash(&self) -> SecretHash {
        let mut sha = Sha256::new();
        sha.input(&self.0);

        let mut result: [u8; SHA256_DIGEST_LENGTH] = [0; SHA256_DIGEST_LENGTH];
        sha.result(&mut result);
        SecretHash(result.to_vec())
    }

    pub fn raw_secret(&self) -> &[u8; SHA256_DIGEST_LENGTH] {
        &self.0
    }
}

impl fmt::LowerHex for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(hex::encode(&self.0).as_str())
    }
}

#[derive(PartialEq, Debug)]
pub enum SecretFromStringErr {
    InvalidLength { expected: usize, got: usize },
    FromHex(hex::FromHexError),
}

impl From<hex::FromHexError> for SecretFromStringErr {
    fn from(err: hex::FromHexError) -> Self {
        SecretFromStringErr::FromHex(err)
    }
}

impl FromStr for Secret {
    type Err = SecretFromStringErr;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let vec = hex::decode(s)?;
        if vec.len() != SHA256_DIGEST_LENGTH {
            return Err(SecretFromStringErr::InvalidLength {
                expected: SHA256_DIGEST_LENGTH,
                got: vec.len(),
            });
        }
        let mut secret = [0; SHA256_DIGEST_LENGTH];
        secret.copy_from_slice(&vec[..]);
        Ok(Secret::from(secret))
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

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
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

impl RandomnessSource for OsRng {
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
    extern crate serde_json;

    #[test]
    fn gen_random_bytes_not_zeros() {
        let mut rng = OsRng::new().unwrap();

        let empty_buf: Vec<u8> = vec![0; 32];
        let buf = rng.gen_random_bytes(32);
        assert_eq!(buf.len(), 32);
        assert_ne!(buf, empty_buf);
    }

    #[test]
    fn new_secret_hash_as_hex() {
        let bytes = b"hello world, you are beautiful!!";
        let mut secret = Secret::from(*bytes);
        assert_eq!(
            secret.hash().to_string(),
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        );
    }

    #[test]
    fn secret_hash_should_be_displayed_as_hex() {
        let bytes = b"hello world, you are beautiful!!";
        let mut secret = Secret::from(*bytes);

        let hash = secret.hash();

        let formatted_hash = format!("{}", hash);

        assert_eq!(
            formatted_hash,
            "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec"
        )
    }

    #[test]
    fn round_trip_secret_serialization() {
        let mut rng = OsRng::new().unwrap();

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
            SecretFromStringErr::InvalidLength {
                expected: 32,
                got: 31
            }
        );
    }
}
