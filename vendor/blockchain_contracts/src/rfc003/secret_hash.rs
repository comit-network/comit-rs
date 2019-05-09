use std::{fmt, str::FromStr};

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SecretHash(Vec<u8>);

impl SecretHash {
    pub const LENGTH: usize = 32;
}

impl fmt::LowerHex for SecretHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(hex::encode(&self.0).as_str())
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

impl From<[u8; SecretHash::LENGTH]> for SecretHash {
    fn from(value: [u8; SecretHash::LENGTH]) -> SecretHash {
        SecretHash(value.to_vec())
    }
}

impl From<SecretHash> for Vec<u8> {
    fn from(secret_hash: SecretHash) -> Self {
        secret_hash.0.to_vec()
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
        Ok(SecretHash(vec))
    }
}
