pub mod asset;
mod dependencies;
pub mod ledger;
pub mod rfc003;

pub mod metadata_store;

pub use self::{
    dependencies::{alice, bob, LedgerEventDependencies},
    ledger::{Ledger, LedgerKind},
    metadata_store::{InMemoryMetadataStore, Metadata, MetadataStore, RoleKind},
};

#[derive(Debug)]
pub enum SwapProtocol {
    Rfc003,
    Unknown(String),
}

mod swap_id;
pub use self::swap_id::*;
