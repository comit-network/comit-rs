use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};
use http_api::asset::{FromHttpAsset, ToHttpAsset};
use std::fmt::Debug;
use swap_protocols::bam_types;

pub trait Asset:
    Clone
    + Debug
    + Send
    + Sync
    + 'static
    + PartialEq
    + FromHttpAsset
    + ToHttpAsset
    + Into<bam_types::Asset>
{
}

impl Asset for BitcoinQuantity {}
impl Asset for EtherQuantity {}
impl Asset for Erc20Quantity {}
