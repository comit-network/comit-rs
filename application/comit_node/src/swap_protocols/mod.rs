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
    metadata_store::{InMemoryMetadataStore, Metadata, MetadataStore, RoleKind},
    swap_id::*,
    timestamp::Timestamp,
};

#[derive(Debug)]
pub enum SwapProtocol {
    Rfc003,
    Unknown(String),
}
