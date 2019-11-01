use crate::{
    seed::Seed,
    swap_protocols::{rfc003::state_store::InMemoryStateStore, InMemoryMetadataStore},
};
use btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use std::sync::Arc;

pub mod alice {
    use super::*;

    #[allow(missing_debug_implementations)]
    pub struct ProtocolDependencies<S> {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<InMemoryMetadataStore>,
        pub state_store: Arc<InMemoryStateStore>,
        pub seed: Seed,
        pub swarm: Arc<S>,
    }

    impl<S> Clone for ProtocolDependencies<S> {
        fn clone(&self) -> Self {
            Self {
                ledger_events: self.ledger_events.clone(),
                metadata_store: Arc::clone(&self.metadata_store),
                state_store: Arc::clone(&self.state_store),
                seed: self.seed,
                swarm: Arc::clone(&self.swarm),
            }
        }
    }
}

pub mod bob {
    use super::*;

    #[allow(missing_debug_implementations)]
    pub struct ProtocolDependencies {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<InMemoryMetadataStore>,
        pub state_store: Arc<InMemoryStateStore>,
        pub seed: Seed,
    }

    impl Clone for ProtocolDependencies {
        fn clone(&self) -> Self {
            Self {
                ledger_events: self.ledger_events.clone(),
                metadata_store: Arc::clone(&self.metadata_store),
                state_store: Arc::clone(&self.state_store),
                seed: self.seed,
            }
        }
    }
}

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct LedgerEventDependencies {
    pub bitcoin_connector: BitcoindConnector,
    pub ethereum_connector: Web3Connector,
}
