pub mod actions;
pub mod ledger;
pub mod rfc003;
mod swap_id;

pub use self::swap_id::*;
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

#[derive(Debug, Clone, Copy)]
pub enum SwapProtocol {
    Rfc003(HashFunction),
}

#[derive(Clone, Copy, Debug, strum_macros::Display, strum_macros::EnumString, PartialEq)]
pub enum Role {
    Alice,
    Bob,
}
