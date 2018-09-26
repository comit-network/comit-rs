use bitcoin_support::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use serde::Serialize;
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use transport_protocol::{json, Status};

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
    Ether { quantity: EthereumQuantity },
}

impl From<BitcoinQuantity> for Asset {
    fn from(quantity: BitcoinQuantity) -> Self {
        Asset::Bitcoin { quantity }
    }
}

impl From<EthereumQuantity> for Asset {
    fn from(quantity: EthereumQuantity) -> Self {
        Asset::Ether { quantity }
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

pub enum SwapResponse<T> {
    Accept(T),
    Decline,
}

impl<T> SwapResponse<T> {
    pub fn status(&self) -> Status {
        match *self {
            SwapResponse::Accept(_) => Status::OK(20),
            SwapResponse::Decline => Status::SE(21),
        }
    }
}

impl<T: Serialize> Into<json::Response> for SwapResponse<T> {
    fn into(self) -> json::Response {
        //TODO: Don't use json::Response but accept a type argument of some trait Response
        let response = json::Response::new(self.status());
        match self {
            SwapResponse::Accept(swap_accept) => response.with_body(swap_accept),
            _ => response,
        }
    }
}
