use crate::ethereum::{Erc20Token, EtherQuantity};
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

impl Asset for EtherQuantity {}

impl Asset for Erc20Token {}

#[derive(Clone, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum AssetKind {
    Bitcoin(Amount),
    Ether(EtherQuantity),
    Erc20(Erc20Token),
}

impl From<Amount> for AssetKind {
    fn from(amount: Amount) -> Self {
        AssetKind::Bitcoin(amount)
    }
}

impl From<EtherQuantity> for AssetKind {
    fn from(quantity: EtherQuantity) -> Self {
        AssetKind::Ether(quantity)
    }
}

impl From<Erc20Token> for AssetKind {
    fn from(quantity: Erc20Token) -> Self {
        AssetKind::Erc20(quantity)
    }
}
