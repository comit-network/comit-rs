mod bitcoin;
pub mod ethereum;
pub mod lightning;
pub use self::{
    bitcoin::Bitcoin,
    ethereum::{Erc20, Erc20Quantity, Ether},
    lightning::Lightning,
};
use crate::asset;
use derivative::Derivative;

#[derive(Clone, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum AssetKind {
    Bitcoin(Bitcoin),
    Ether(Ether),
    Erc20(Erc20),
}

impl From<Bitcoin> for AssetKind {
    fn from(amount: Bitcoin) -> Self {
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
