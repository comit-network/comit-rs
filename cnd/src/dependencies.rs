use crate::{
    seed::{Seed, SwapSeed},
    swap_protocols::{
        metadata_store::InMemoryMetadataStore, rfc003::state_store::InMemoryStateStore,
        LedgerConnectors, SwapId,
    },
};
use std::sync::Arc;

/// Core cnd dependencies required by both the libp2p network layer and the HTTP
/// API layer.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Dependencies {
    pub ledger_events: LedgerConnectors,
    pub metadata_store: Arc<InMemoryMetadataStore>,
    pub state_store: Arc<InMemoryStateStore>,
    pub seed: Seed,
}

impl SwapSeed for Dependencies {
    fn swap_seed(&self, id: SwapId) -> Seed {
        self.seed.swap_seed(id)
    }
}
