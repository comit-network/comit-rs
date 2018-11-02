use bitcoin_support::BitcoinQuantity;
use ethereum_support::EtherQuantity;
use std::fmt::Debug;
use swap_protocols::wire_types;

pub trait Asset:
    Clone + Debug + Send + Sync + 'static + PartialEq + Into<wire_types::Asset>
{
}

impl Asset for BitcoinQuantity {}
impl Asset for EtherQuantity {}
