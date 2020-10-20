use crate::swap::{herc20, LedgerTime};
use comit::{
    btsieve::{ethereum::Web3Connector, LatestBlock},
    Timestamp,
};
use std::{sync::Arc, time::Duration};

pub use comit::{
    ethereum::{Address, Block, ChainId, Hash, Transaction},
    Secret,
};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Arc<crate::ethereum::Wallet>,
    pub connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    pub gas_price: crate::ethereum::GasPrice,
}

#[async_trait::async_trait]
impl herc20::ExecuteDeploy for Wallet {
    async fn execute_deploy(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let action = params.build_deploy_action();
        let gas_price = self
            .gas_price
            .gas_price(0) // Note on `0`: this impl-block is going away
            .await?;
        let deployed_contract = self.inner.deploy_contract(action, gas_price).await?;

        Ok(deployed_contract.into())
    }
}

#[async_trait::async_trait]
impl herc20::ExecuteFund for Wallet {
    async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> anyhow::Result<herc20::Funded> {
        let action = params.build_fund_action(deploy_event.location);
        let gas_price = self
            .gas_price
            .gas_price(0) // Note on `0`: this impl-block is going away
            .await?;
        let _data = self.inner.call_contract(action, gas_price).await?;

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
        utc_start_of_swap: OffsetDateTime,
    ) -> anyhow::Result<herc20::Redeemed> {
        let action = params.build_redeem_action(deploy_event.location, secret);
        let gas_price = self
            .gas_price
            .gas_price(0) // Note on `0`: this impl-block is going away
            .await?;
        let _data = self.inner.call_contract(action, gas_price).await?;

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
        utc_start_of_swap: OffsetDateTime,
    ) -> anyhow::Result<herc20::Refunded> {
        loop {
            if self.ledger_time().await? >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let action = params.build_refund_action(deploy_event.location);
        let gas_price = self
            .gas_price
            .gas_price(0) // Note on `0`: this impl-block is going away
            .await?;
        let _data = self.inner.call_contract(action, gas_price).await?;

        let event =
            herc20::watch_for_refunded(self.connector.as_ref(), utc_start_of_swap, deploy_event)
                .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl LedgerTime for Web3Connector {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        comit::ethereum::latest_time(self).await
    }
}

#[async_trait::async_trait]
impl LedgerTime for Wallet {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.connector.as_ref().ledger_time().await
    }
}

#[async_trait::async_trait]
impl LatestBlock for Wallet {
    type Block = Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.latest_block().await
    }
}
