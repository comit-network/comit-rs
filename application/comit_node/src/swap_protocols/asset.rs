use crate::http_api::asset::{FromHttpAsset, ToHttpAsset};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Token, EtherQuantity};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};
pub trait Asset:
    Clone
    + Debug
    + Display
    + Send
    + Sync
    + 'static
    + PartialEq
    + Eq
    + Hash
    + FromHttpAsset
    + ToHttpAsset
    + Into<AssetKind>
{
}

impl Asset for BitcoinQuantity {}
impl Asset for EtherQuantity {}
impl Asset for Erc20Token {}

#[derive(Debug, Clone)]
pub enum AssetKind {
    Bitcoin(BitcoinQuantity),
    Ether(EtherQuantity),
    Erc20(Erc20Token),
    Unknown { name: String },
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
