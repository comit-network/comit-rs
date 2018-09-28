use bigdecimal::ParseBigDecimalError;
use serde::{
    de::{self, Deserialize, Deserializer},
    ser::{Serialize, Serializer},
};
use std::{
    fmt,
    ops::{Add, Sub},
    str::FromStr,
};

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct BitcoinQuantity(u64);

impl BitcoinQuantity {
    pub fn from_satoshi(sats: u64) -> Self {
        BitcoinQuantity(sats)
    }
    pub fn from_bitcoin(btc: f64) -> Self {
        BitcoinQuantity((btc * 100_000_000.0).round() as u64)
    }
    pub fn satoshi(self) -> u64 {
        self.0
    }
    pub fn bitcoin(self) -> f64 {
        (self.0 as f64) / 100_000_000.0
    }
}

impl Add for BitcoinQuantity {
    type Output = BitcoinQuantity;

    fn add(self, rhs: BitcoinQuantity) -> BitcoinQuantity {
        BitcoinQuantity(self.0 + rhs.0)
    }
}

impl Sub for BitcoinQuantity {
    type Output = BitcoinQuantity;

    fn sub(self, rhs: BitcoinQuantity) -> BitcoinQuantity {
        BitcoinQuantity(self.0 - rhs.0)
    }
}

impl fmt::Display for BitcoinQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} BTC", self.bitcoin())
    }
}

impl FromStr for BitcoinQuantity {
    type Err = ParseBigDecimalError;

    fn from_str(string: &str) -> Result<BitcoinQuantity, Self::Err> {
        let dec = string.parse()?;
        Ok(Self::from_bitcoin(dec))
    }
}

impl<'de> Deserialize<'de> for BitcoinQuantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'vde> de::Visitor<'vde> for Visitor {
            type Value = BitcoinQuantity;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                formatter.write_str("A string representing a satoshi quantity")
            }

            fn visit_str<E>(self, v: &str) -> Result<BitcoinQuantity, E>
            where
                E: de::Error,
            {
                Ok(v.parse()
                    .map(BitcoinQuantity::from_satoshi)
                    .map_err(E::custom)?)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for BitcoinQuantity {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}
