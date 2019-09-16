pub mod actions;
pub mod asset;
mod dependencies;
pub mod ledger;
pub mod metadata_store;
pub mod rfc003;
mod swap_id;
mod timestamp;

pub use self::{
    dependencies::{alice, bob, LedgerEventDependencies},
    ledger::{Ledger, LedgerKind},
    metadata_store::{Metadata, MetadataStore, Role},
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
