use crypto::{digest::Digest, sha2::Sha256};
use hex;
use rand::{Rng, ThreadRng};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

pub const SECRET_LENGTH: usize = 32;

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SecretHash(pub Vec<u8>);

impl Debug for SecretHash {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&format!("SecretHash({:x})", self))
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Secret([u8; SECRET_LENGTH]);

impl From<[u8; SECRET_LENGTH]> for Secret {
    fn from(secret: [u8; SECRET_LENGTH]) -> Self {
        Secret(secret)
    }
}

impl From<Secret> for SecretHash {
    fn from(secret: Secret) -> Self {
        secret.hash()
    }
}

impl Secret {
    pub fn generate<T: RandomnessSource>(rng: &mut T) -> Secret {
        let random_bytes = rng.gen_random_bytes(SECRET_LENGTH);
        let mut secret = [0; SECRET_LENGTH];
        secret.copy_from_slice(&random_bytes[..]);
        Secret::from(secret)
    }

    pub fn from_vec(vec: &[u8]) -> Result<Secret, SecretFromErr> {
        if vec.len() != SECRET_LENGTH {
            return Err(SecretFromErr::InvalidLength {
                expected: SECRET_LENGTH,
                got: vec.len(),
            });
        }
        let mut data = [0; SECRET_LENGTH];
        let vec = &vec[..SECRET_LENGTH];
        data.copy_from_slice(vec);
        Ok(Secret(data))
    }

    pub fn hash(&self) -> SecretHash {
        let mut sha = Sha256::new();
        sha.input(&self.0);

        let mut result: [u8; SECRET_LENGTH] = [0; SECRET_LENGTH];
        sha.result(&mut result);
        SecretHash(result.to_vec())
    }

    pub fn raw_secret(&self) -> &[u8; SECRET_LENGTH] {
        &self.0
    }
}

impl fmt::LowerHex for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(hex::encode(&self.0).as_str())
    }
}

#[derive(PartialEq, Debug)]
pub enum SecretFromErr {
    InvalidLength { expected: usize, got: usize },
    FromHex(hex::FromHexError),
}

impl From<hex::FromHexError> for SecretFromErr {
    fn from(err: hex::FromHexError) -> Self {
        SecretFromErr::FromHex(err)
    }
}

impl FromStr for Secret {
    type Err = SecretFromErr;

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
    use rand;
    use serde_json;
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
            SecretFromErr::InvalidLength {
                expected: 32,
                got: 31
            }
        );
    }
}
