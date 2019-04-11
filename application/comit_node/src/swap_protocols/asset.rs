use crate::http_api::asset::FromHttpAsset;
use bitcoin_support::BitcoinQuantity;
use derivative::Derivative;
use ethereum_support::{Erc20Token, EtherQuantity};
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
    + FromHttpAsset
    + Into<AssetKind>
{
    fn equal_or_greater_value(&self, other: &Self) -> bool;
}

impl Asset for BitcoinQuantity {
    fn equal_or_greater_value(&self, other: &BitcoinQuantity) -> bool {
        self >= other
    }
}
impl Asset for EtherQuantity {
    fn equal_or_greater_value(&self, other: &EtherQuantity) -> bool {
        self >= other
    }
}
impl Asset for Erc20Token {
    fn equal_or_greater_value(&self, other: &Erc20Token) -> bool {
        self.token_contract == other.token_contract && self.quantity >= other.quantity
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug = "transparent")]
pub enum AssetKind {
    Bitcoin(BitcoinQuantity),
    Ether(EtherQuantity),
    Erc20(Erc20Token),
    Unknown(String),
}

impl From<BitcoinQuantity> for AssetKind {
    fn from(quantity: BitcoinQuantity) -> Self {
        AssetKind::Bitcoin(quantity)
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
