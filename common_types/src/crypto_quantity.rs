use num::FromPrimitive;
use rust_decimal;
use rust_decimal::Decimal;
pub use rust_decimal::Error as EthereumConversionError;
use std::fmt;
use std::ops::{Add, Sub};
use std::str::FromStr;
use web3::types::U256;

#[derive(Serialize, PartialEq, Deserialize, Clone, Debug, Copy)]
pub struct BitcoinQuantity(u64);

pub trait CurrencyQuantity {
    fn nominal_amount(&self) -> f64;
    fn from_nominal_amount(nominal_amount: f64) -> Self;
}

impl CurrencyQuantity for BitcoinQuantity {
    fn nominal_amount(&self) -> f64 {
        self.bitcoin()
    }
    fn from_nominal_amount(bitcoin: f64) -> Self {
        Self::from_bitcoin(bitcoin)
    }
}

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

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub struct EthereumQuantity(U256);

lazy_static! {
    static ref WEI_IN_ETHEREUM: U256 = U256::from((10 as u64).pow(18));
}

impl CurrencyQuantity for EthereumQuantity {
    fn nominal_amount(&self) -> f64 {
        self.ethereum()
    }
    fn from_nominal_amount(ethereum: f64) -> Self {
        Self::from_eth(ethereum)
    }
}

impl EthereumQuantity {
    fn extract_significand(decimal: Decimal) -> U256 {
        let ser = decimal.serialize();
        let _flags = &ser[0..4];
        let little_endian_int_data = &ser[4..16];
        let mut buf = [0u8; 32];
        // ignore first 4 bytes which contain meta info
        buf[0..12].clone_from_slice(little_endian_int_data);
        buf.reverse(); // convert big endian -- can probably redesigned to avoid this
        buf.into()
    }

    fn convert_significand_to_wei(significand: U256, scale: u32) -> U256 {
        U256::from((10 as u64).pow(18 - scale)) * significand
    }

    fn decimal_to_wei(decimal: Decimal) -> U256 {
        let significand = Self::extract_significand(decimal);
        Self::convert_significand_to_wei(significand, decimal.scale())
    }

    pub fn from_eth(eth: f64) -> Self {
        let dec =
            Decimal::from_f64(eth).expect(format!("{} is an invalid eth value", eth).as_str());
        EthereumQuantity(Self::decimal_to_wei(dec))
    }

    pub fn from_wei(wei: U256) -> Self {
        EthereumQuantity(wei)
    }

    pub fn ethereum(&self) -> f64 {
        unimplemented!()
    }

    pub fn wei(&self) -> U256 {
        U256::from(self.0)
    }
}

impl fmt::Display for EthereumQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} wei (ETH)", self.wei())
    }
}

impl FromStr for EthereumQuantity {
    type Err = rust_decimal::Error;
    fn from_str(string: &str) -> Result<EthereumQuantity, Self::Err> {
        let dec = Decimal::from_str(string)?;
        Ok(EthereumQuantity(Self::decimal_to_wei(dec)))
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
    fn bitcoin_from_nominal_amount_is_the_same_as_from_botcoin() {
        assert_eq!(
            BitcoinQuantity::from_bitcoin(1.0),
            BitcoinQuantity::from_nominal_amount(1.0)
        );
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

    #[test]
    fn display_ethereum() {
        assert_eq!(
            format!("{}", EthereumQuantity::from_wei(9000.into())),
            "9000 wei (ETH)"
        );
    }

    #[test]
    fn a_ethereum_is_a_quintillion_wei() {
        assert_eq!(
            EthereumQuantity::from_eth(2.0).wei(),
            U256::from(2_000_000_000_000_000_000 as u64) // 2 quintillion
        )
    }

    #[test]
    fn from_eth_works_when_resulting_wei_cant_fit_in_u64() {
        assert_eq!(
            EthereumQuantity::from_eth(9001.0).wei(),
            U256::from(9001 as u64) * *WEI_IN_ETHEREUM
        )
    }

    #[test]
    fn from_fractional_ethereum_converts_to_correct_wei() {
        assert_eq!(
            EthereumQuantity::from_eth(0.000_000_001).wei(),
            U256::from(1_000_000_000)
        )
    }

    #[test]
    fn ethereum_quantity_from_str() {
        assert_eq!(
            EthereumQuantity::from_str("1.000_000_001").unwrap().wei(),
            U256::from(1_000_000_001_000_000_000 as u64)
        )
    }

}
