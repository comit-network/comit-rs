use crate::{
    ledger_query_service::{DefaultLedgerQueryServiceApiClient, FirstMatch, QueryIdCache},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        metadata_store::{self, Metadata, MetadataStore},
        rfc003::{
            bob::SwapRequest,
            events::{LedgerEvents, LqsEvents, LqsEventsForErc20, ResponseFuture},
            state_store::{self, StateStore},
            Bob, Initiation, Ledger,
        },
        SwapId,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};
use futures::Future;
use std::{sync::Arc, time::Duration};

#[derive(Debug)]
pub enum Error {
    Storage(state_store::Error),
    Metadata(metadata_store::Error),
}

#[allow(missing_debug_implementations)]
pub struct BobSpawner<T, S> {
    lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
}

impl<T: MetadataStore<SwapId>, S: StateStore<SwapId>> BobSpawner<T, S> {
    pub fn new(
        lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
        metadata_store: Arc<T>,
        state_store: Arc<S>,
        bitcoin_poll_interval: Duration,
        ethereum_poll_interval: Duration,
    ) -> Self {
        Self {
            lqs_client,
            bitcoin_poll_interval,
            ethereum_poll_interval,
            metadata_store,
            state_store,
        }
    }

    pub fn spawn_bob<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        id: SwapId,
        swap_request: SwapRequest<AL, BL, AA, BA>,
    ) -> Result<Box<ResponseFuture<Bob<AL, BL, AA, BA>>>, Error>
    where
        Self: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        SwapRequest<AL, BL, AA, BA>: Into<Metadata>,
    {
        let save_state = self
            .state_store
            .new_save_state(id)
            .map_err(Error::Storage)?;
        self.metadata_store
            .insert(id, swap_request.clone())
            .map_err(Error::Metadata)?;

        let initiation = Initiation {
            alpha_asset: swap_request.alpha_asset,
            beta_asset: swap_request.beta_asset,
            alpha_ledger: swap_request.alpha_ledger,
            beta_ledger: swap_request.beta_ledger,
            beta_ledger_redeem_identity: swap_request.beta_ledger_redeem_identity,
            alpha_ledger_refund_identity: swap_request.alpha_ledger_refund_identity,
            alpha_ledger_lock_duration: swap_request.alpha_ledger_lock_duration,
            secret: swap_request.secret_hash,
        };

        let (state_machine_future, response_future) = Bob::new_state_machine(
            initiation,
            self.create_ledger_events(),
            self.create_ledger_events(),
            save_state,
        );

        tokio::spawn(
            state_machine_future
                .map(move |outcome| {
                    info!("Swap {} finished with {:?}", id, outcome);
                })
                .map_err(move |e| {
                    error!("Swap {} failed with {:?}", id, e);
                }),
        );

        Ok(response_future)
    }
}

pub trait CreateLedgerEvents<L: Ledger, A: Asset> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<L, A>>;
}

impl<T, S> CreateLedgerEvents<Bitcoin, BitcoinQuantity> for BobSpawner<T, S> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Bitcoin, BitcoinQuantity>> {
        Box::new(LqsEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.bitcoin_poll_interval),
        ))
    }
}

impl<T, S> CreateLedgerEvents<Ethereum, EtherQuantity> for BobSpawner<T, S> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, EtherQuantity>> {
        Box::new(LqsEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.ethereum_poll_interval),
        ))
    }
}

impl<T, S> CreateLedgerEvents<Ethereum, Erc20Quantity> for BobSpawner<T, S> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, Erc20Quantity>> {
        Box::new(LqsEventsForErc20::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.ethereum_poll_interval),
        ))
    }
}
