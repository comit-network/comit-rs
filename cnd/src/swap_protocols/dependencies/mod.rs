use crate::{seed::Seed, swap_protocols::InMemoryMetadataStore};
use btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use std::sync::Arc;

mod client_impl;

pub mod alice {
    use super::*;

    #[allow(missing_debug_implementations)]
    pub struct ProtocolDependencies<S, C> {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<InMemoryMetadataStore>,
        pub state_store: Arc<S>,
        pub seed: Seed,
        pub client: Arc<C>,
    }

    impl<S, C> Clone for ProtocolDependencies<S, C> {
        fn clone(&self) -> Self {
            Self {
                ledger_events: self.ledger_events.clone(),
                metadata_store: Arc::clone(&self.metadata_store),
                state_store: Arc::clone(&self.state_store),
                seed: self.seed,
                client: Arc::clone(&self.client),
            }
        }
    }
}

pub mod bob {
    use super::*;

    #[allow(missing_debug_implementations)]
    pub struct ProtocolDependencies<S> {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<InMemoryMetadataStore>,
        pub state_store: Arc<S>,
        pub seed: Seed,
    }

    impl<S> Clone for ProtocolDependencies<S> {
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
