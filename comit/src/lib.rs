// Cannot do `#[strum_discriminants(derive(strum_macros::EnumString))]` at the
// moment. Hence we need to `#[macro_use]` in order to derive strum macros on
// an enum created by `strum_discriminants`.
#[macro_use]
extern crate strum_macros;

pub mod actions;
pub mod asset;
pub mod ledger;
pub mod rfc003;

mod client;
mod swap_id;
mod timestamp;

pub use self::{
    asset::{Asset, AssetKind},
    client::{Client, RequestError},
    ledger::{Ledger, LedgerKind},
    swap_id::*,
    timestamp::Timestamp,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Copy)]
pub enum HashFunction {
    #[serde(rename = "SHA-256")]
    Sha256,
}

#[derive(Debug)]
pub enum SwapProtocol {
    Rfc003(HashFunction),
    Unknown(String),
}
