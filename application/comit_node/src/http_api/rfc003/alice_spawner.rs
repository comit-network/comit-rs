use crate::{
    comit_client::{self, ClientFactory},
    ledger_query_service::{DefaultLedgerQueryServiceApiClient, FirstMatch, QueryIdCache},
    swap_protocols::{
        asset::Asset,
        ledger::{Bitcoin, Ethereum},
        metadata_store::{self, Metadata, MetadataStore},
        rfc003::{
            alice::SwapRequest,
            events::{LedgerEvents, LqsEvents, LqsEventsForErc20},
            state_store::{self, StateStore},
            Alice, Initiation, Ledger, SecretSource,
        },
        SwapId,
    },
};
use bitcoin_support::BitcoinQuantity;
use ethereum_support::{Erc20Quantity, EtherQuantity};
use futures::Future;
use http_api_problem::HttpApiProblem;
use std::{net::SocketAddr, sync::Arc, time::Duration};

#[allow(missing_debug_implementations)]
pub struct AliceSpawner<C> {
    remote_comit_node: SocketAddr,
    lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
    comit_client_factory: Arc<dyn ClientFactory<C>>,
    secret_source: Arc<dyn SecretSource>,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
}

#[derive(Debug)]
pub enum SpawnError {
    Storage(state_store::Error),
    Metadata(metadata_store::Error),
}

impl From<SpawnError> for HttpApiProblem {
    fn from(e: SpawnError) -> Self {
        use self::SpawnError::*;
        match e {
            Storage(e) => e.into(),
            Metadata(e) => e.into(),
        }
    }
}

impl<C: comit_client::Client> AliceSpawner<C> {
    pub fn new(
        remote_comit_node: SocketAddr,
        lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
        comit_client_factory: Arc<dyn ClientFactory<C>>,
        secret_source: Arc<dyn SecretSource>,
        bitcoin_poll_interval: Duration,
        ethereum_poll_interval: Duration,
    ) -> Self {
        Self {
            lqs_client,
            remote_comit_node,
            comit_client_factory,
            secret_source,
            bitcoin_poll_interval,
            ethereum_poll_interval,
        }
    }

    pub fn spawn_alice<
        AL: Ledger,
        BL: Ledger,
        AA: Asset,
        BA: Asset,
        T: MetadataStore<SwapId>,
        S: StateStore<SwapId>,
    >(
        &self,
        id: SwapId,
        swap_request: SwapRequest<AL, BL, AA, BA>,
        metadata_store: &T,
        state_store: &S,
    ) -> Result<(), SpawnError>
    where
        Self: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        SwapRequest<AL, BL, AA, BA>: Into<Metadata>,
    {
        let save_state = state_store
            .new_save_state(id)
            .map_err(SpawnError::Storage)?;
        metadata_store
            .insert(id, swap_request.clone())
            .map_err(SpawnError::Metadata)?;

        let initiation = Initiation {
            alpha_asset: swap_request.alpha_asset,
            beta_asset: swap_request.beta_asset,
            alpha_ledger: swap_request.alpha_ledger,
            beta_ledger: swap_request.beta_ledger,
            beta_ledger_redeem_identity: swap_request.identities.beta_ledger_redeem_identity,
            alpha_ledger_refund_identity: swap_request.identities.alpha_ledger_refund_identity,
            alpha_ledger_lock_duration: swap_request.alpha_ledger_lock_duration,
            secret: self.secret_source.new_secret(id),
        };

        let state_machine_future = Alice::<AL, BL, AA, BA>::new_state_machine(
            initiation,
            self.create_ledger_events(),
            self.create_ledger_events(),
            Arc::clone(&self.comit_client_factory),
            self.remote_comit_node,
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

        Ok(())
    }
}

pub trait CreateLedgerEvents<L: Ledger, A: Asset> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<L, A>>;
}

impl<C> CreateLedgerEvents<Bitcoin, BitcoinQuantity> for AliceSpawner<C> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Bitcoin, BitcoinQuantity>> {
        Box::new(LqsEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.bitcoin_poll_interval),
        ))
    }
}

impl<C> CreateLedgerEvents<Ethereum, EtherQuantity> for AliceSpawner<C> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, EtherQuantity>> {
        Box::new(LqsEvents::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.ethereum_poll_interval),
        ))
    }
}

impl<C> CreateLedgerEvents<Ethereum, Erc20Quantity> for AliceSpawner<C> {
    fn create_ledger_events(&self) -> Box<dyn LedgerEvents<Ethereum, Erc20Quantity>> {
        Box::new(LqsEventsForErc20::new(
            QueryIdCache::wrap(Arc::clone(&self.lqs_client)),
            FirstMatch::new(Arc::clone(&self.lqs_client), self.ethereum_poll_interval),
        ))
    }
}
