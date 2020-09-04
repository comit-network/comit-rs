use crate::asset;
use std::{fmt, str::FromStr};

mod text;
mod u32;

pub use self::{text::Text, u32::U32};
use comit::{asset::Erc20Quantity, Price, Quantity};

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
    fn from(btc: asset::Bitcoin) -> Self {
        Satoshis(btc.as_sat())
    }
}

impl From<Satoshis> for asset::Bitcoin {
    fn from(value: Satoshis) -> asset::Bitcoin {
        asset::Bitcoin::from_sat(value.0)
    }
}

impl From<Text<Satoshis>> for Quantity<asset::Bitcoin> {
    fn from(s: Text<Satoshis>) -> Self {
        Quantity::new(s.0.into())
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

#[derive(Debug, Clone, PartialEq)]
pub struct WeiPerSat(asset::Erc20Quantity);

impl FromStr for WeiPerSat {
    type Err = crate::asset::ethereum::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        asset::Erc20Quantity::from_wei_dec_str(s).map(Self)
    }
}

impl fmt::Display for WeiPerSat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_wei_dec())
    }
}

impl From<Text<WeiPerSat>> for Price<asset::Bitcoin, Erc20Quantity> {
    fn from(rate: Text<WeiPerSat>) -> Self {
        Price::from_wei_per_sat((rate.0).0)
    }
}
