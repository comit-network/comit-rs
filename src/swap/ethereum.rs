use crate::swap::{herc20, BetaLedgerTime};
use chrono::NaiveDateTime;
use comit::{btsieve::LatestBlock, Timestamp};
use std::{sync::Arc, time::Duration};

pub use comit::{
    ethereum::{Address, Block, ChainId, Hash},
    Secret,
};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: crate::ethereum_wallet::Wallet,
    pub connector: Arc<comit::btsieve::ethereum::Web3Connector>,
}

impl Wallet {
    pub async fn refund(&self, action: herc20::CallContract) -> anyhow::Result<()> {
        let _ = self.inner.call_contract(action).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl herc20::ExecuteDeploy for Wallet {
    async fn execute_deploy(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let action = params.build_deploy_action();
        let transaction_hash = self.inner.deploy_contract(action).await?;
        let transaction = self.inner.get_transaction_by_hash(transaction_hash).await?;

        let receipt = self.inner.get_transaction_receipt(transaction_hash).await?;
        let location = receipt
            .contract_address
            .ok_or_else(|| anyhow::anyhow!("Contract address missing from receipt"))?;

        Ok(herc20::Deployed {
            transaction,
            location,
        })
    }
}

#[async_trait::async_trait]
impl herc20::ExecuteFund for Wallet {
    async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Funded> {
        let action = params.build_fund_action(deploy_event.location);
        let _data = self.inner.call_contract(action).await?;

        let event =
            herc20::watch_for_funded(self.connector.as_ref(), params, start_of_swap, deploy_event)
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
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Redeemed> {
        let action = params.build_redeem_action(deploy_event.location, secret);
        let _data = self.inner.call_contract(action).await?;

        let event =
            herc20::watch_for_redeemed(self.connector.as_ref(), start_of_swap, deploy_event)
                .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl herc20::ExecuteRefund for Wallet {
    async fn execute_refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Refunded> {
        loop {
            if self.beta_ledger_time().await? >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let action = params.build_refund_action(deploy_event.location);
        let _data = self.inner.call_contract(action).await?;

        let event =
            herc20::watch_for_refunded(self.connector.as_ref(), start_of_swap, deploy_event)
                .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<C> BetaLedgerTime for C
where
    C: LatestBlock<Block = Block>,
{
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
        ethereum_latest_time(self).await
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
