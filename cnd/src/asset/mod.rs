mod erc20;
mod ether;
pub use self::{
    erc20::{Erc20, Erc20Quantity},
    ether::Ether,
};

use crate::asset;
use bitcoin::Amount;
use derivative::Derivative;
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

pub trait Asset:
    Clone
    + Copy
    + Debug
    + Display
    + Send
    + Sync
    + 'static
    + PartialEq
    + Eq
    + Hash
    + Into<AssetKind>
    + Ord
{
}

impl Asset for Amount {}

impl Asset for Ether {}

impl Asset for Erc20 {}

#[derive(Clone, Copy, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum AssetKind {
    Bitcoin(Amount),
    Ether(Ether),
    Erc20(Erc20),
}

impl From<Amount> for AssetKind {
    fn from(amount: Amount) -> Self {
        AssetKind::Bitcoin(amount)
    }
}

impl From<asset::Ether> for AssetKind {
    fn from(quantity: asset::Ether) -> Self {
        AssetKind::Ether(quantity)
    }
}

impl From<asset::Erc20> for AssetKind {
    fn from(quantity: asset::Erc20) -> Self {
        AssetKind::Erc20(quantity)
    }
}
