use crate::{
    btsieve::{QueryBitcoin, QueryEthereum},
    seed::Seed,
};
use std::sync::Arc;

mod client_impl;
mod server_impl;

pub mod alice {
    use super::*;
    use crate::{
        network::Behaviour,
        swap_protocols::{rfc003::state_store::StateStore, swap_id::SwapId, MetadataStore},
    };
    use libp2p::{Swarm, Transport};
    use std::sync::Mutex;
    use tokio::{io::AsyncRead, prelude::AsyncWrite};

    #[allow(missing_debug_implementations)]
    pub struct ProtocolDependencies<T, S, TTransport, TSubstream>
    where
        TTransport: Transport + Send + 'static,
        TSubstream: AsyncRead + AsyncWrite + Send + 'static,
        T: MetadataStore<SwapId>,
        S: StateStore,
    {
        pub ledger_events: LedgerEventDependencies,
        pub metadata_store: Arc<T>,
        pub state_store: Arc<S>,
        pub seed: Seed,
        pub swarm: Arc<Mutex<Swarm<TTransport, Behaviour<TSubstream, T, S>>>>,
    }

    impl<T, S, TTransport, TSubstream> Clone for ProtocolDependencies<T, S, TTransport, TSubstream>
    where
        TTransport: Transport + Send + 'static,
        TSubstream: AsyncRead + AsyncWrite + Send + 'static,
        T: MetadataStore<SwapId>,
        S: StateStore,
    {
        fn clone(&self) -> Self {
            Self {
                ledger_events: self.ledger_events.clone(),
                metadata_store: Arc::clone(&self.metadata_store),
                state_store: Arc::clone(&self.state_store),
                seed: self.seed.clone(),
                swarm: Arc::clone(&self.swarm),
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
