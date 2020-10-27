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
use comit::swap::herc20::{IncorrectlyFunded, WatchForDeployed, WatchForFunded, WatchForRedeemed};
use std::sync::Arc;
use time::OffsetDateTime;

pub use comit::herc20::*;

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
        loop {
            match watch_for_deployed(self.connector.as_ref(), params.clone(), utc_start_of_swap)
                .await
            {
                Ok(event) => {
                    self.storage
                        .herc20_events
                        .lock()
                        .await
                        .entry(self.swap_id)
                        .or_default()
                        .deploy = Some(event);
                    return event;
                }
                Err(e) => tracing::warn!(
                    "failed to watch for herc20 deployment, retrying ...: {:#}",
                    e
                ),
            }
        }
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
        loop {
            match watch_for_funded(
                self.connector.as_ref(),
                params.clone(),
                utc_start_of_swap,
                deploy_event,
            )
            .await
            {
                Ok(comit::herc20::Funded::Correctly { transaction, asset }) => {
                    let event = comit::swap::herc20::Funded { transaction, asset };
                    self.storage
                        .herc20_events
                        .lock()
                        .await
                        .entry(self.swap_id)
                        .or_default()
                        .fund = Some(event.clone());

                    return Ok(event);
                }
                Ok(comit::herc20::Funded::Incorrectly { asset, .. }) => {
                    return Err(IncorrectlyFunded {
                        expected: params.asset,
                        got: asset,
                    })
                }
                Err(e) => {
                    tracing::warn!("failed to watch for herc20 funding, retrying ...: {:#}", e)
                }
            }
        }
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
        loop {
            match watch_for_redeemed(self.connector.as_ref(), utc_start_of_swap, deploy_event).await
            {
                Ok(event) => {
                    self.storage
                        .herc20_events
                        .lock()
                        .await
                        .entry(self.swap_id)
                        .or_default()
                        .redeem = Some(event);

                    return event;
                }
                Err(e) => {
                    tracing::warn!("failed to watch for herc20 funding, retrying ...: {:#}", e)
                }
            }
        }
    }
}
