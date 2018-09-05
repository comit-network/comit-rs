use bitcoin_support::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use serde::Serialize;
use transport_protocol::{json, Status};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "value", content = "parameters")]
pub enum Ledger {
    Bitcoin,
    Ethereum,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "value", content = "parameters")]
pub enum Asset {
    Bitcoin { quantity: BitcoinQuantity },
    Ether { quantity: EthereumQuantity },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "value", content = "parameters")]
pub enum SwapProtocol {
    #[serde(rename = "COMIT-RFC-003")]
    ComitRfc003,
}

#[derive(Debug, PartialEq)]
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
            SwapResponse::Decline => Status::OK(21),
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
