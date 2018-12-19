use crate::{
    comit_client::ClientFactory, ledger_query_service::DefaultLedgerQueryServiceApiClient,
    seed::Seed,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};

/// Represents the things you have have access to when starting execution of a
/// protocol
#[allow(missing_debug_implementations)]
pub struct ProtocolDependencies<T, S, C> {
    pub ledger_events: LedgerEventDependencies,
    pub metadata_store: Arc<T>,
    pub state_store: Arc<S>,
    pub comit_client_factory: Arc<dyn ClientFactory<C>>,
    pub seed: Seed,
    pub remote_comit_node: SocketAddr,
}

#[derive(Debug)]
pub struct LedgerEventDependencies {
    pub lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
    pub lqs_bitcoin_poll_interval: Duration,
    pub lqs_ethereum_poll_interval: Duration,
}
