use bigdecimal::ParseBigDecimalError;
use std::{
    fmt,
    ops::{Add, Sub},
    str::FromStr,
};

#[derive(Serialize, PartialEq, Deserialize, Clone, Debug, Copy)]
pub struct BitcoinQuantity(u64);

impl BitcoinQuantity {
    pub fn from_satoshi(sats: u64) -> Self {
        BitcoinQuantity(sats)
    }
    pub fn from_bitcoin(btc: f64) -> Self {
        BitcoinQuantity((btc * 100_000_000.0).round() as u64)
    }
    pub fn satoshi(&self) -> u64 {
        self.0
    }
    pub fn bitcoin(&self) -> f64 {
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
