use ethereum_support::{FromDecimalStr, U256};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, derive_more::FromStr, derive_more::Display)]
pub struct Satoshis(pub u64);

/// The `FromStr` implementation of U256 expects hex but we want to store
/// decimal numbers in the database to aid human-readability.
///
/// This type wraps U256 to provide `FromStr` and `Display` implementations that
/// use decimal numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimalU256(pub U256);

impl FromStr for DecimalU256 {
    type Err = <ethereum_support::U256 as FromDecimalStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U256::from_decimal_str(s).map(DecimalU256)
    }
}

impl fmt::Display for DecimalU256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EthereumAddress(pub ethereum_support::Address);

impl FromStr for EthereumAddress {
    type Err = <ethereum_support::Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(EthereumAddress)
    }
}

impl fmt::Display for EthereumAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}
