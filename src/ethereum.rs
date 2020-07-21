pub mod dai;
mod geth;
mod wallet;

pub use comit::ethereum::{Address, ChainId, Hash};
pub use geth::Client;
pub use wallet::Wallet;

pub const STANDARD_ETH_TRANSFER_GAS_LIMIT: u64 = 21_000;

pub mod ether {
    use crate::float_maths::multiply_pow_ten;
    use anyhow::Context;
    use comit::asset::ethereum::{FromWei, TryFromWei};
    use comit::asset::Ether;
    use comit::ethereum::U256;
    use num::{BigUint, Num};
    use num256::Uint256;
    use std::convert::{TryFrom, TryInto};
    use std::fmt;
    use std::str::FromStr;

    const WEI_IN_ETHER_EXP: u16 = 18;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Amount(comit::asset::ethereum::Ether);

    impl Amount {
        pub fn zero() -> Self {
            Self(comit::asset::ethereum::Ether::zero())
        }

        pub fn try_from_hex(hex: String) -> anyhow::Result<Self> {
            let hex = if hex.starts_with("0x") {
                &hex.as_str()[2..]
            } else {
                hex.as_str()
            };

            let int = BigUint::from_str_radix(hex, 16)?;
            let amount = comit::asset::ethereum::Ether::try_from_wei(int)?;

            Ok(Self(amount))
        }

        /// Smallest accepted unit is wei.
        pub fn from_ether_str(ether: &str) -> anyhow::Result<Self> {
            let u_int_value = multiply_pow_ten(ether, WEI_IN_ETHER_EXP as u16)
                .context("The value passed is not valid for ether")?;

            u_int_value.try_into()
        }
    }

    impl TryFrom<BigUint> for Amount {
        type Error = anyhow::Error;

        fn try_from(int: BigUint) -> Result<Self, Self::Error> {
            Ok(Amount(comit::asset::Ether::try_from_wei(int)?))
        }
    }

    impl From<comit::asset::ethereum::Ether> for Amount {
        fn from(ether: Ether) -> Self {
            Amount(ether)
        }
    }

    impl From<Amount> for Uint256 {
        fn from(amount: Amount) -> Self {
            Uint256::from_bytes_le(&amount.0.to_bytes())
        }
    }

    impl From<Amount> for U256 {
        fn from(amount: Amount) -> Self {
            amount.0.to_u256()
        }
    }

    /// Integer is wei
    impl From<u64> for Amount {
        fn from(int: u64) -> Self {
            Amount(comit::asset::ethereum::Ether::from_wei(int))
        }
    }

    /// Accepts decimal string of wei
    impl FromStr for Amount {
        type Err = comit::asset::ethereum::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let ether = comit::asset::ethereum::Ether::from_wei_dec_str(s)?;
            Ok(Amount(ether))
        }
    }

    impl fmt::Display for Amount {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Currently testing the comit crate but best to state expectations here.
        // Also give the opportunity to have tests if we were to decides to implement our own
        // representation.
        #[test]
        fn one_ether_shows_as_one_ether() {
            let ether = Amount::from_str("1_000_000_000_000_000_000").unwrap();

            assert_eq!(ether.to_string(), "1 ETH")
        }

        #[test]
        fn ten_ether_shows_as_ten_ether() {
            let ether = Amount::from_str("10_000_000_000_000_000_000").unwrap();

            assert_eq!(ether.to_string(), "10 ETH")
        }

        #[test]
        fn one_wei_shows_as_atto_ether() {
            let ether = Amount::from_str("1").unwrap();

            assert_eq!(ether.to_string(), "0.000000000000000001 ETH")
        }
    }
}
