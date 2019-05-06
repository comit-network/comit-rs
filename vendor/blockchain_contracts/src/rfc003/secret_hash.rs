use std::{fmt, str::FromStr};

// TODO: Replace with Vec<u8>
#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct SecretHash(String);

impl SecretHash {
    pub const LENGTH: usize = 32;
}

impl fmt::LowerHex for SecretHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(&self.0.to_lowercase())
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
        Ok(SecretHash(s.to_string()))
    }
}
