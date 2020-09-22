pub mod dai;
mod geth;
mod wallet;

pub use comit::ethereum::{Address, ChainId, Hash};
pub use geth::Client;
pub use wallet::Wallet;

pub const STANDARD_ETH_TRANSFER_GAS_LIMIT: u64 = 21_000;
pub const DAI_TRANSFER_GAS_LIMIT: u64 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    Mainnet,
    Ropsten,
    Rinkeby,
    Kovan,
    Local {
        chain_id: u32,
        dai_contract_address: Address,
    },
}

impl Chain {
    pub fn new(chain_id: ChainId, dai_contract_address: Address) -> Self {
        use Chain::*;
        match (chain_id.into(), dai_contract_address) {
            (1, contract) if dai::is_mainnet_contract_address(contract) => Mainnet,
            (3, contract) if dai::is_ropsten_contract_address(contract) => Ropsten,
            (4, contract) if dai::is_rinkeby_contract_address(contract) => Rinkeby,
            (42, contract) if dai::is_kovan_contract_address(contract) => Kovan,
            (chain_id, dai_contract_address) => Local {
                chain_id,
                dai_contract_address,
            },
        }
    }

    pub fn from_public_chain_id(chain_id: ChainId) -> anyhow::Result<Self> {
        use Chain::*;
        match chain_id.into() {
            1 => Ok(Mainnet),
            3 => Ok(Ropsten),
            4 => Ok(Rinkeby),
            42 => Ok(Kovan),
            _ => anyhow::bail!("chain_id does not correspond to public chain"),
        }
    }

    pub fn dai_contract_address(&self) -> Address {
        dai::token_contract_address(*self)
    }

    pub fn chain_id(&self) -> ChainId {
        use Chain::*;
        match self {
            Mainnet => ChainId::MAINNET,
            Ropsten => ChainId::ROPSTEN,
            Rinkeby => ChainId::from(4),
            Kovan => ChainId::from(42),
            Local { chain_id, .. } => ChainId::from(*chain_id),
        }
    }
}

#[cfg(test)]
impl crate::StaticStub for Chain {
    fn static_stub() -> Self {
        Chain::Mainnet
    }
}

pub mod ether {
    use crate::float_maths::multiply_pow_ten;
    use anyhow::Context;
    use clarity::Uint256;
    use comit::{
        asset::{
            ethereum::{FromWei, TryFromWei},
            Ether,
        },
        ethereum::U256,
    };
    use num::{BigUint, Num};
    use std::{
        convert::{TryFrom, TryInto},
        fmt,
        str::FromStr,
    };

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
        // Also give the opportunity to have tests if we were to decides to implement
        // our own representation.
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
