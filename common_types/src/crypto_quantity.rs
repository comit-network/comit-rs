use std::fmt;
use std::ops::{Add, Sub};
use web3::types::U256;

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

impl fmt::Display for BitcoinQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} BTC", self.bitcoin())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub struct EthereumQuantity(u64);

lazy_static! {
    static ref WEI_IN_ETHEREUM: U256 = U256::from((10 as u64).pow(18));
}

impl EthereumQuantity {
    pub fn from_eth(eth: u64) -> Self {
        EthereumQuantity(eth)
    }

    pub fn ethereum(&self) -> u64 {
        self.0
    }

    pub fn wei(&self) -> U256 {
        U256::from(self.0) * *WEI_IN_ETHEREUM
    }
}

impl fmt::Display for EthereumQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} ETH", self.ethereum())
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

    #[test]
    fn display_bitcoin() {
        assert_eq!(format!("{}", BitcoinQuantity::from_bitcoin(42)), "42 BTC");
        assert_eq!(
            format!("{}", BitcoinQuantity::from_satoshi(200_000_000)),
            "2 BTC"
        );
    }

    #[test]
    fn display_ethereum() {
        assert_eq!(format!("{}", EthereumQuantity::from_eth(9000)), "9000 ETH");
    }

    #[test]
    fn a_ethereum_is_a_quintillion_wei() {
        assert_eq!(
            EthereumQuantity::from_eth(2).wei(),
            *WEI_IN_ETHEREUM * U256::from(2)
        )
    }

}
