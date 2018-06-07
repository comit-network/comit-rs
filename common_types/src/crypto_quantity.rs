use std::ops::{Add, Sub};

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub struct BitcoinQuantity(u64);

impl BitcoinQuantity {
    pub fn from_satoshi(sats: u64) -> Self {
        BitcoinQuantity(sats)
    }
    pub fn from_bitcoin(btc: u64) -> Self {
        BitcoinQuantity(btc * 100_000_000)
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EthQuantity(u64);

impl EthQuantity {
    pub fn from_eth(eth: u64) -> Self {
        EthQuantity(eth)
    }

    pub fn eth(&self) -> u64 {
        self.0
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
        assert_eq!(BitcoinQuantity::from_bitcoin(1).satoshi(), 100_000_000);
    }
}
