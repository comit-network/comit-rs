use crate::seed::Seed;
use btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use std::sync::Arc;

mod client_impl;

pub mod alice {
    use super::*;

    #[allow(missing_debug_implementations)]
    pub struct ProtocolDependencies<T, S, C> {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<T>,
        pub state_store: Arc<S>,
        pub seed: Seed,
        pub client: Arc<C>,
    }

    impl<T, S, C> Clone for ProtocolDependencies<T, S, C> {
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
    #[derive(Clone)]
    pub struct ProtocolDependencies<T, S> {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<T>,
        pub state_store: Arc<S>,
        pub seed: Seed,
    }
}

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct LedgerEventDependencies {
    pub bitcoin_blocksource: Arc<BitcoindConnector>,
    pub ethereum_blocksource: Arc<Web3Connector>,
}
