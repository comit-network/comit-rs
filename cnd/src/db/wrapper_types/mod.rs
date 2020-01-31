use crate::db::LedgerKind;
use comit::{asset, asset::Bitcoin, ethereum, swap_protocols::ledger};
use std::{fmt, str::FromStr};

pub mod custom_sql_types;

/// A wrapper type for representing satoshis
///
/// Together with the `Text` sql type, this will store the number as a string to
/// avoid precision loss.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Satoshis(u64);

impl FromStr for Satoshis {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str(s).map(Self)
    }
}

impl fmt::Display for Satoshis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<asset::Bitcoin> for Satoshis {
    fn from(btc: Bitcoin) -> Self {
        Satoshis(btc.as_sat())
    }
}

impl From<Satoshis> for asset::Bitcoin {
    fn from(value: Satoshis) -> asset::Bitcoin {
        asset::Bitcoin::from_sat(value.0)
    }
}

/// These types wrap Ethereum assets to provide `FromStr` and `Display`
/// implementations that use decimal numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct Ether(asset::Ether);

impl From<Ether> for asset::Ether {
    fn from(value: Ether) -> asset::Ether {
        value.0
    }
}

impl From<asset::Ether> for Ether {
    fn from(asset: asset::Ether) -> Ether {
        Ether(asset)
    }
}

impl FromStr for Ether {
    type Err = comit::asset::ethereum::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        asset::Ether::from_wei_dec_str(s).map(Ether)
    }
}

impl fmt::Display for Ether {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_wei_dec())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Erc20Amount(asset::Erc20Quantity);

impl From<Erc20Amount> for asset::Erc20Quantity {
    fn from(value: Erc20Amount) -> asset::Erc20Quantity {
        value.0
    }
}

impl From<asset::Erc20Quantity> for Erc20Amount {
    fn from(asset: asset::Erc20Quantity) -> Erc20Amount {
        Erc20Amount(asset)
    }
}

impl FromStr for Erc20Amount {
    type Err = comit::asset::ethereum::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        asset::Erc20Quantity::from_wei_dec_str(s).map(Erc20Amount)
    }
}

impl fmt::Display for Erc20Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_wei_dec())
    }
}

/// A wrapper type for ethereum addresses.
///
/// Together with the `Text` sql type, this will store an ethereum address in
/// hex encoding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EthereumAddress(ethereum::Address);

impl FromStr for EthereumAddress {
    type Err = <ethereum::Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(EthereumAddress)
    }
}

impl fmt::Display for EthereumAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl From<EthereumAddress> for ethereum::Address {
    fn from(address: EthereumAddress) -> Self {
        address.0
    }
}

impl From<ethereum::Address> for EthereumAddress {
    fn from(address: ethereum::Address) -> Self {
        EthereumAddress(address)
    }
}

/// A wrapper type for Bitcoin networks.
///
/// This is then wrapped in the db::custom_sql_types::Text to be stored in DB
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet,
    Regtest,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum Error {
    #[error("Unknown variant")]
    Unknown,
}

impl FromStr for BitcoinNetwork {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            "regtest" => Ok(Self::Regtest),
            _ => Err(Error::Unknown),
        }
    }
}

impl fmt::Display for BitcoinNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Regtest => "regtest",
        };
        write!(f, "{}", s)
    }
}

macro_rules! impl_from_for_bitcoinnetwork {
    ($ledger:ident) => {
        impl From<$ledger> for BitcoinNetwork {
            fn from(_: $ledger) -> Self {
                BitcoinNetwork::$ledger
            }
        }
    };
}

use ledger::bitcoin::{Mainnet, Regtest, Testnet};
impl_from_for_bitcoinnetwork!(Mainnet);
impl_from_for_bitcoinnetwork!(Testnet);
impl_from_for_bitcoinnetwork!(Regtest);

impl From<BitcoinNetwork> for LedgerKind {
    fn from(network: BitcoinNetwork) -> Self {
        match network {
            BitcoinNetwork::Mainnet => LedgerKind::BitcoinMainnet,
            BitcoinNetwork::Testnet => LedgerKind::BitcoinTestnet,
            BitcoinNetwork::Regtest => LedgerKind::BitcoinRegtest,
        }
    }
}
