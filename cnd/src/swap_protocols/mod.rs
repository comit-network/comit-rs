pub mod actions;
pub mod asset;
pub mod ledger;
pub mod metadata_store;
pub mod rfc003;
mod swap_id;
mod timestamp;

pub use self::{
    ledger::{Ledger, LedgerConnectors, LedgerKind},
    metadata_store::{InMemoryMetadataStore, Metadata, MetadataStore},
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

#[derive(Clone, Copy, Debug, Display, EnumString)]
pub enum Role {
    Alice,
    Bob,
}
