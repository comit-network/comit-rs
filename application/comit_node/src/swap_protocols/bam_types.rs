use bam::Status;
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};
use serde::Serialize;
use swap_protocols::ledger::{Bitcoin, Ethereum};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "value", content = "parameters")]
pub enum Ledger {
    Bitcoin,
    Ethereum,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "value", content = "parameters")]
pub enum Asset {
    Bitcoin { quantity: BitcoinQuantity },
    Ether { quantity: EtherQuantity },
    Erc20 { quantity: Erc20Quantity },
}

impl From<BitcoinQuantity> for Asset {
    fn from(quantity: BitcoinQuantity) -> Self {
        Asset::Bitcoin { quantity }
    }
}

impl From<EtherQuantity> for Asset {
    fn from(quantity: EtherQuantity) -> Self {
        Asset::Ether { quantity }
    }
}

impl From<Erc20Quantity> for Asset {
    fn from(quantity: Erc20Quantity) -> Self {
        Asset::Erc20 { quantity }
    }
}

impl From<Bitcoin> for Ledger {
    fn from(_: Bitcoin) -> Self {
        Ledger::Bitcoin
    }
}

impl From<Ethereum> for Ledger {
    fn from(_: Ethereum) -> Self {
        Ledger::Ethereum
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "value", content = "parameters")]
pub enum SwapProtocol {
    #[serde(rename = "COMIT-RFC-003")]
    ComitRfc003,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SwapRequestHeaders {
    pub source_ledger: Ledger,
    pub target_ledger: Ledger,
    pub source_asset: Asset,
    pub target_asset: Asset,
    pub swap_protocol: SwapProtocol,
}

#[derive(Debug)]
pub enum SwapResponse {
    Accept,
    Decline,
}

impl SwapResponse {
    pub fn status(&self) -> Status {
        match *self {
            SwapResponse::Accept => Status::OK(20),
            SwapResponse::Decline => Status::SE(21),
        }
    }
}
