use crate::asset;
use std::{fmt, str::FromStr};

/// A new type for representing satoshis
///
/// Together with the `Text` sql type, this will store the number as a string to
/// avoid precision loss.
#[derive(Debug, Clone, Copy, PartialEq, derive_more::FromStr, derive_more::Display)]
pub struct Satoshis(pub u64);

impl From<Satoshis> for u64 {
    fn from(value: Satoshis) -> u64 {
        value.0
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
    type Err = crate::asset::ethereum::Error;

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
    type Err = crate::asset::ethereum::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        asset::Erc20Quantity::from_wei_dec_str(s).map(Erc20Amount)
    }
}

impl fmt::Display for Erc20Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_wei_dec())
    }
}

/// A new type for ethereum addresses.
///
/// Together with the `Text` sql type, this will store an ethereum address in
/// hex encoding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EthereumAddress(pub crate::ethereum::Address);

impl FromStr for EthereumAddress {
    type Err = <crate::ethereum::Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(EthereumAddress)
    }
}

impl fmt::Display for EthereumAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}
