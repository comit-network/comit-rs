use hex::{FromHex, FromHexError};
use serde::{de, de::Visitor, Deserializer};
use std::{fmt, marker::PhantomData};

/// A deserializer for 0x prefixed hex-strings
pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromHex<Error = FromHexError>,
{
    struct HexVisitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for HexVisitor<T>
    where
        T: FromHex<Error = FromHexError>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a 0x-prefixed hex-string")
        }

        fn visit_str<E>(self, v: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            if v.len() >= 2 && &v[0..2] == "0x" {
                T::from_hex(&v[2..]).map_err(|e| match e {
                    FromHexError::InvalidHexCharacter { c, index } => E::invalid_value(
                        de::Unexpected::Char(c),
                        &format!("Unexpected character {:?} as position {}", c, index).as_str(),
                    ),
                    FromHexError::InvalidStringLength => {
                        E::invalid_length(v.len(), &"Unexpected length of hex string")
                    }
                    FromHexError::OddLength => {
                        E::invalid_length(v.len(), &"Odd length of hex string")
                    }
                })
            } else {
                Err(serde::de::Error::custom("invalid format"))
            }
        }
    }

    deserializer.deserialize_str(HexVisitor(PhantomData))
}
