use crate::{
    bam_api::header::{FromBamHeader, ToBamHeader},
    http_api::asset::{FromHttpAsset, ToHttpAsset},
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};
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
    + FromBamHeader
    + ToBamHeader
{
}

impl Asset for BitcoinQuantity {}
impl Asset for EtherQuantity {}
impl Asset for Erc20Quantity {}
