use bigdecimal;
use bigdecimal::BigDecimal;
use byteorder::{LittleEndian, WriteBytesExt};
use num::FromPrimitive;
use num::ToPrimitive;
use num::bigint::{BigInt, Sign};
use regex::Regex;
use std::f64;
use std::fmt;
use std::mem;
use std::ops::{Add, Mul, Sub};
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

impl CurrencyQuantity for EthereumQuantity {
    fn nominal_amount(&self) -> f64 {
        self.ethereum()
    }
    fn from_nominal_amount(ethereum: f64) -> Self {
        Self::from_eth(ethereum)
    }
}

const U64SIZE: usize = mem::size_of::<u64>();

impl EthereumQuantity {
    fn bigdecimal_eth_to_u256_wei(decimal: BigDecimal) -> U256 {
        let (wei_bigint, _) = decimal.with_scale(18).as_bigint_and_exponent();
        let (_sign, bytes) = wei_bigint.to_bytes_be();
        let mut buf = [0u8; 32];
        let start = 32 - bytes.len();
        // ignore first 4 bytes which contain meta info
        buf[start..].clone_from_slice(&bytes[..]);
        buf.into()
    }

    pub fn from_eth(eth: f64) -> Self {
        let dec =
            BigDecimal::from_f64(eth).expect(format!("{} is an invalid eth value", eth).as_str());
        EthereumQuantity(Self::bigdecimal_eth_to_u256_wei(dec))
    }

    pub fn from_wei(wei: U256) -> Self {
        EthereumQuantity(wei)
    }

    fn to_ethereum_bigdec(&self) -> BigDecimal {
        let mut bs = [0u8; U64SIZE * 4];

        let _u256 = self.0;
        let four_u64s_little_endian = _u256.0;

        for index in 0..4 {
            let _u64 = four_u64s_little_endian[index];
            let start = index * U64SIZE;
            let end = (index + 1) * U64SIZE;
            bs[start..end]
                .as_mut()
                .write_u64::<LittleEndian>(_u64)
                .expect("Unable to write");
        }

        let bigint = BigInt::from_bytes_le(Sign::Plus, &bs);

        BigDecimal::new(bigint, 18)
    }

    pub fn ethereum(&self) -> f64 {
        self.to_ethereum_bigdec().to_f64().unwrap()
    }

    pub fn wei(&self) -> U256 {
        U256::from(self.0)
    }
}

lazy_static! {
    static ref TRAILING_ZEROS: Regex = Regex::new(r"\.?0*$").unwrap();
}

impl fmt::Display for EthereumQuantity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // At time of writing BigDecimal always puts . and pads zeroes
        // up to the precision in f, so TRAILING_ZEROS does the right
        // thing in all cases.
        let fmt_dec = format!("{}", self.to_ethereum_bigdec());
        let removed_trailing_zeros = TRAILING_ZEROS.replace(fmt_dec.as_str(), "");
        write!(f, "{} ETH", removed_trailing_zeros)
    }
}

impl FromStr for EthereumQuantity {
    type Err = bigdecimal::ParseBigDecimalError;
    fn from_str(string: &str) -> Result<EthereumQuantity, Self::Err> {
        let dec = BigDecimal::from_str(string)?;
        Ok(EthereumQuantity(Self::bigdecimal_eth_to_u256_wei(dec)))
    }
}

#[cfg(test)]
mod test {
    lazy_static! {
        static ref WEI_IN_ETHEREUM: U256 = U256::from((10u64).pow(18));
    }

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
            format!("{}", EthereumQuantity::from_eth(9000.0)),
            "9000 ETH"
        );
    }

    #[test]
    fn a_ethereum_is_a_quintillion_wei() {
        assert_eq!(
            EthereumQuantity::from_eth(2.0).wei(),
            U256::from(2_000_000_000_000_000_000u64) // 2 quintillion
        )
    }

    #[test]
    fn from_eth_works_when_resulting_wei_cant_fit_in_u64() {
        assert_eq!(
            EthereumQuantity::from_eth(9001.0).wei(),
            U256::from(9001u64) * *WEI_IN_ETHEREUM
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
            EthereumQuantity::from_str("1.000000001").unwrap().wei(),
            U256::from(1_000_000_001_000_000_000u64)
        )
    }

    #[test]
    fn ethereum_quantity_back_into_f64() {
        assert!(EthereumQuantity::from_eth(0.1234).ethereum() - 0.1234f64 < f64::EPSILON)
    }

    #[test]
    fn fractional_ethereum_format() {
        assert_eq!(
            format!("{}", EthereumQuantity::from_eth(0.1234)),
            "0.1234 ETH"
        )
    }

    #[test]
    fn whole_ethereum_format() {
        assert_eq!(format!("{}", EthereumQuantity::from_eth(12.0)), "12 ETH");
    }

    #[test]
    fn ethereum_with_small_fraction_format() {
        assert_eq!(
            format!("{}", EthereumQuantity::from_str("1234.00000100").unwrap()),
            "1234.000001 ETH"
        )
    }

    #[test]
    fn one_hundren_ethereum_format() {
        assert_eq!(
            format!("{}", EthereumQuantity::from_str("100").unwrap()),
            "100 ETH"
        )
    }

}
