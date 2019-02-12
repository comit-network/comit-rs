use crate::{
    connection_pool::ConnectionPool,
    ledger_query_service::{QueryBitcoin, QueryEthereum},
    seed::Seed,
};
use std::sync::Arc;

mod client_impl;

/// Represents the things you have access to when starting execution of a
/// protocol
#[allow(missing_debug_implementations)]
pub struct ProtocolDependencies<T, S> {
    pub ledger_events: LedgerEventDependencies,
    pub metadata_store: Arc<T>,
    pub state_store: Arc<S>,
    pub connection_pool: Arc<ConnectionPool>,
    pub seed: Seed,
}

impl<T, S> Clone for ProtocolDependencies<T, S> {
    fn clone(&self) -> Self {
        ProtocolDependencies {
            ledger_events: self.ledger_events.clone(),
            metadata_store: Arc::clone(&self.metadata_store),
            state_store: Arc::clone(&self.state_store),
            connection_pool: Arc::clone(&self.connection_pool),
            seed: self.seed,
        }
    }
}

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct LedgerEventDependencies {
    pub query_bitcoin: Arc<dyn QueryBitcoin + Send + Sync + 'static>,
    pub query_ethereum: Arc<dyn QueryEthereum + Send + Sync + 'static>,
}

impl<Q: QueryBitcoin + QueryEthereum + Send + Sync + 'static> From<Q> for LedgerEventDependencies {
    fn from(querier: Q) -> Self {
        let queries = Arc::new(querier);
        LedgerEventDependencies {
            query_bitcoin: queries.clone(),
            query_ethereum: queries.clone(),
        }
    }
}
