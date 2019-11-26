pub mod actions;
pub mod asset;
pub mod ledger;
pub mod rfc003;
mod swap_id;
mod timestamp;

pub use self::{
    ledger::{Ledger, LedgerConnectors, LedgerKind},
    swap_id::*,
    timestamp::Timestamp,
};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Copy,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
pub enum HashFunction {
    #[strum(serialize = "SHA-256")]
    #[serde(rename = "SHA-256")]
    Sha256,
}

#[derive(Debug)]
pub enum SwapProtocol {
    Rfc003(HashFunction),
    Unknown(String),
}

#[derive(Clone, Copy, Debug, Display, EnumString, PartialEq)]
pub enum Role {
    Alice,
    Bob,
}
