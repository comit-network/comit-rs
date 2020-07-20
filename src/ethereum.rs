pub mod dai;
mod geth;
mod wallet;

pub use geth::Client;
pub use wallet::Wallet;

pub mod ether {
    use comit::asset::ethereum::TryFromWei;
    use comit::asset::Ether;
    use num::{BigUint, Num};
    use std::fmt;
    use std::str::FromStr;

    const WEI_IN_ETHER_EXP: usize = 18;

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
    }

    impl From<comit::asset::ethereum::Ether> for Amount {
        fn from(ether: Ether) -> Self {
            Amount(ether)
        }
    }

    /// Accepts decimal string
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
