pub use comit::herc20::*;

use crate::{
    btsieve::{
        ethereum::{GetLogs, ReceiptByHash, TransactionByHash},
        BlockByHash, ConnectedNetwork, LatestBlock,
    },
    ethereum::{Block, ChainId, Hash},
    storage::Storage,
    LocalSwapId,
};
use anyhow::Result;
use backoff::{backoff::Constant, future::FutureOperation};
use comit::swap::herc20::{IncorrectlyFunded, WatchForDeployed, WatchForFunded, WatchForRedeemed};
use futures::TryFutureExt;
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;

#[derive(Clone, Debug, Default)]
pub struct Events {
    pub deploy: Option<comit::herc20::Deployed>,
    pub fund: Option<comit::swap::herc20::Funded>,
    pub redeem: Option<comit::herc20::Redeemed>,
}

pub struct Facade<C> {
    pub connector: Arc<C>,
    pub swap_id: LocalSwapId,
    pub storage: Storage,
}

#[async_trait::async_trait]
impl<C> WatchForDeployed for Facade<C>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + ConnectedNetwork<Network = ChainId>,
{
    async fn watch_for_deployed(
        &self,
        params: Params,
        utc_start_of_swap: OffsetDateTime,
    ) -> Deployed {
        let operation = || {
            watch_for_deployed(self.connector.as_ref(), params.clone(), utc_start_of_swap)
                .map_err(backoff::Error::Transient)
        };

        let deployed = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!(
                    "failed to watch for herc20 deployment, retrying ...: {:#}",
                    e
                )
            })
            .await
            .expect("transient error is never returned");

        self.storage
            .herc20_events
            .lock()
            .await
            .entry(self.swap_id)
            .or_default()
            .deploy = Some(deployed);

        deployed
    }
}

#[async_trait::async_trait]
impl<C> WatchForFunded for Facade<C>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + TransactionByHash
        + ConnectedNetwork<Network = ChainId>
        + GetLogs,
{
    async fn watch_for_funded(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Result<comit::swap::herc20::Funded, IncorrectlyFunded> {
        let operation = || {
            watch_for_funded(
                self.connector.as_ref(),
                params.clone(),
                utc_start_of_swap,
                deploy_event,
            )
            .map_err(backoff::Error::Transient)
        };

        let funded = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for herc20 funding, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned")?;

        self.storage
            .herc20_events
            .lock()
            .await
            .entry(self.swap_id)
            .or_default()
            .fund = Some(funded.clone());

        Ok(funded)
    }
}

#[async_trait::async_trait]
impl<C> WatchForRedeemed for Facade<C>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + TransactionByHash
        + ConnectedNetwork<Network = ChainId>
        + GetLogs,
{
    async fn watch_for_redeemed(
        &self,
        _: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Redeemed {
        let operation = || {
            watch_for_redeemed(self.connector.as_ref(), utc_start_of_swap, deploy_event)
                .map_err(backoff::Error::Transient)
        };

        let redeemed = operation
            .retry_notify(Constant::new(Duration::from_secs(1)), |e, _| {
                tracing::warn!("failed to watch for herc20 redeem, retrying ...: {:#}", e)
            })
            .await
            .expect("transient error is never returned");

        self.storage
            .herc20_events
            .lock()
            .await
            .entry(self.swap_id)
            .or_default()
            .redeem = Some(redeemed);

        redeemed
    }
}
