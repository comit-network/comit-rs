use std::{
    fmt,
    ops::{Add, Sub},
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hundred_million_sats_is_a_bitcoin() {
        assert_eq!(BitcoinQuantity::from_satoshi(100_000_000).bitcoin(), 1.0);
    }

    #[test]
    fn a_bitcoin_is_a_hundred_million_sats() {
        assert_eq!(BitcoinQuantity::from_bitcoin(1.0).satoshi(), 100_000_000);
    }

    #[test]
    fn display_bitcoin() {
        assert_eq!(format!("{}", BitcoinQuantity::from_bitcoin(42.0)), "42 BTC");
        assert_eq!(
            format!("{}", BitcoinQuantity::from_satoshi(200_000_000)),
            "2 BTC"
        );
    }
}
