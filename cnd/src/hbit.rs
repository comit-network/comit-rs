pub use comit::{hbit::*, identity};

use crate::{
    btsieve::{BlockByHash, ConnectedNetwork, LatestBlock},
    ledger,
    storage::Storage,
    LocalSwapId,
};
use anyhow::Result;
use backoff::{backoff::Constant, future::FutureOperation};
use comit::swap::hbit::{IncorrectlyFunded, WatchForFunded, WatchForRedeemed};
use futures::TryFutureExt;
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;

#[derive(Clone, Debug, Default)]
pub struct Events {
    pub fund: Option<comit::swap::hbit::Funded>,
    pub redeem: Option<comit::hbit::Redeemed>,
}

pub struct Facade<C> {
    pub connector: Arc<C>,
    pub swap_id: LocalSwapId,
    pub storage: Storage,
}

#[async_trait::async_trait]
impl<C> WatchForFunded for Facade<C>
where
    C: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    async fn watch_for_funded(
        &self,
        params: &Params,
        start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded> {
        let operation = || {
            comit::hbit::watch_for_funded(self.connector.as_ref(), &params.shared, start_of_swap)
                .map_err(backoff::Error::Transient)
        };

        let funded = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for hbit funding, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned")?;

        self.storage
            .hbit_events
            .lock()
            .await
            .entry(self.swap_id)
            .or_default()
            .fund = Some(funded);

        Ok(funded)
    }
}

#[async_trait::async_trait]
impl<C> WatchForRedeemed for Facade<C>
where
    C: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>
        + ConnectedNetwork<Network = ledger::Bitcoin>,
{
    async fn watch_for_redeemed(
        &self,
        params: &Params,
        fund_event: Funded,
        start_of_swap: OffsetDateTime,
    ) -> Redeemed {
        let operation = || {
            watch_for_redeemed(
                self.connector.as_ref(),
                &params.shared,
                fund_event.location,
                start_of_swap,
            )
            .map_err(backoff::Error::Transient)
        };

        let redeemed = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for hbit redeem, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned");

        self.storage
            .hbit_events
            .lock()
            .await
            .entry(self.swap_id)
            .or_default()
            .redeem = Some(redeemed);

        redeemed
    }
}
