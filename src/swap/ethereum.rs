use crate::swap::{herc20, LedgerTime};
use chrono::{DateTime, Utc};
use comit::{
    btsieve::{ethereum::Web3Connector, LatestBlock},
    Timestamp,
};
use std::{sync::Arc, time::Duration};

pub use comit::{
    ethereum::{Address, Block, ChainId, Hash, Transaction},
    Secret,
};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Arc<crate::ethereum::Wallet>,
    pub connector: Arc<comit::btsieve::ethereum::Web3Connector>,
}

#[async_trait::async_trait]
impl herc20::ExecuteDeploy for Wallet {
    async fn execute_deploy(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let action = params.build_deploy_action();
        let deployed_contract = self.inner.deploy_contract(action).await?;

        Ok(deployed_contract.into())
    }
}

#[async_trait::async_trait]
impl herc20::ExecuteFund for Wallet {
    async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        utc_start_of_swap: DateTime<Utc>,
    ) -> anyhow::Result<herc20::Funded> {
        let action = params.build_fund_action(deploy_event.location);
        let _data = self.inner.call_contract(action).await?;

        let event = herc20::watch_for_funded(
            self.connector.as_ref(),
            params,
            utc_start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl herc20::ExecuteRedeem for Wallet {
    async fn execute_redeem(
        &self,
        params: herc20::Params,
        secret: Secret,
        deploy_event: herc20::Deployed,
        utc_start_of_swap: DateTime<Utc>,
    ) -> anyhow::Result<herc20::Redeemed> {
        let action = params.build_redeem_action(deploy_event.location, secret);
        let _data = self.inner.call_contract(action).await?;

        let event =
            herc20::watch_for_redeemed(self.connector.as_ref(), utc_start_of_swap, deploy_event)
                .await?;

        Ok(event)
    }
}

/// Trigger the refund path of the HTLC corresponding to the
/// `herc20::Params` and the `herc20::Deployed` event passed, once
/// it's possible.
#[async_trait::async_trait]
impl herc20::ExecuteRefund for Wallet {
    async fn execute_refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        utc_start_of_swap: DateTime<Utc>,
    ) -> anyhow::Result<herc20::Refunded> {
        loop {
            if self.ledger_time().await? >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let action = params.build_refund_action(deploy_event.location);
        let _data = self.inner.call_contract(action).await?;

        let event =
            herc20::watch_for_refunded(self.connector.as_ref(), utc_start_of_swap, deploy_event)
                .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl LedgerTime for Web3Connector {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        ethereum_latest_time(self).await
    }
}

#[async_trait::async_trait]
impl LedgerTime for Wallet {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.connector.as_ref().ledger_time().await
    }
}

async fn ethereum_latest_time<C>(connector: &C) -> anyhow::Result<Timestamp>
where
    C: LatestBlock<Block = Block>,
{
    let timestamp = connector.latest_block().await?.timestamp.into();

    Ok(timestamp)
}

#[async_trait::async_trait]
impl LatestBlock for Wallet {
    type Block = Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.latest_block().await
    }
}
