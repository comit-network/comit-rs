use crate::{
    btsieve::{BlockByHash, ConnectedNetwork, LatestBlock},
    ledger,
    storage::Storage,
    LocalSwapId,
};
use anyhow::Result;
use comit::swap::hbit::{IncorrectlyFunded, WatchForFunded, WatchForRedeemed};
use std::sync::Arc;
use time::OffsetDateTime;

pub use comit::{hbit::*, identity};

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
        loop {
            match watch_for_funded(self.connector.as_ref(), &params.shared, start_of_swap).await {
                Ok(Ok(event)) => {
                    self.storage
                        .hbit_events
                        .lock()
                        .await
                        .entry(self.swap_id)
                        .or_default()
                        .fund = Some(event);

                    return Ok(event);
                }
                Ok(Err(e)) => return Err(e),
                Err(e) => tracing::warn!("failed to watch for hbit funding, retrying ...: {:#}", e),
            }
        }
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
        loop {
            match watch_for_redeemed(
                self.connector.as_ref(),
                &params.shared,
                fund_event.location,
                start_of_swap,
            )
            .await
            {
                Ok(event) => {
                    self.storage
                        .hbit_events
                        .lock()
                        .await
                        .entry(self.swap_id)
                        .or_default()
                        .redeem = Some(event);

                    return event;
                }
                Err(e) => tracing::warn!("failed to watch for hbit redeem, retrying ...: {:#}", e),
            }
        }
    }
}
