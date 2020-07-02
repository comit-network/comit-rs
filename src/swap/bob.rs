//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::swap::{
    bitcoin, db,
    ethereum::{self, ethereum_latest_time},
    Next, {hbit, herc20},
};
use chrono::NaiveDateTime;
use comit::{Secret, SecretHash, Timestamp};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW, DB, E> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: DB,
    pub private_protocol_details: E,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
}

#[async_trait::async_trait]
impl<AW, DB, E> herc20::Deploy for WalletBob<AW, ethereum::Wallet, DB, E>
where
    AW: Send + Sync,
    DB: db::Load<herc20::Deployed> + db::Save<herc20::Deployed>,
    E: Send + Sync,
{
    async fn deploy(
        &self,
        params: herc20::Params,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<herc20::Deployed>> {
        {
            if let Some(deploy_event) = self.db.load(0).await? {
                return Ok(Next::Continue(deploy_event));
            }

            let beta_ledger_time = ethereum_latest_time(&self.beta_wallet).await?;
            if beta_expiry <= beta_ledger_time {
                return Ok(Next::Abort);
            }

            let deploy_event = self.deploy(&params).await?;
            self.db.save(deploy_event.clone(), 0).await?;

            Ok(Next::Continue(deploy_event))
        }
    }
}

#[async_trait::async_trait]
impl<AW, DB, E> herc20::Fund for WalletBob<AW, ethereum::Wallet, DB, E>
where
    AW: Send + Sync,
    DB: db::Load<herc20::CorrectlyFunded> + db::Save<herc20::CorrectlyFunded>,
    E: Send + Sync,
{
    async fn fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<herc20::CorrectlyFunded>> {
        if let Some(fund_event) = self.db.load(0).await? {
            return Ok(Next::Continue(fund_event));
        }

        let beta_ledger_time = ethereum_latest_time(&self.beta_wallet).await?;
        if beta_expiry <= beta_ledger_time {
            return Ok(Next::Abort);
        }

        let fund_event = self.fund(params, deploy_event).await?;
        self.db.save(fund_event.clone(), 0).await?;

        Ok(Next::Continue(fund_event))
    }
}

#[async_trait::async_trait]
impl<DB> hbit::RedeemAsBob
    for WalletBob<bitcoin::Wallet, ethereum::Wallet, DB, hbit::PrivateDetailsRedeemer>
where
    DB: Send + Sync,
{
    async fn redeem(
        &self,
        params: &hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        secret: Secret,
    ) -> anyhow::Result<hbit::Redeemed> {
        self.redeem(*params, fund_event, secret).await
    }
}

#[async_trait::async_trait]
impl<DB> herc20::Refund
    for WalletBob<bitcoin::Wallet, ethereum::Wallet, DB, hbit::PrivateDetailsRedeemer>
where
    DB: Send + Sync,
{
    async fn refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Refunded> {
        loop {
            let ethereum_time = ethereum_latest_time(self.beta_wallet.connector.as_ref()).await?;

            if ethereum_time >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_event = self.refund(params, deploy_event).await?;

        Ok(refund_event)
    }
}

impl<AW, DB, E> WalletBob<AW, ethereum::Wallet, DB, E> {
    async fn deploy(&self, params: &herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let deploy_action = params.build_deploy_action();
        let event = self.beta_wallet.deploy(deploy_action).await?;

        Ok(event)
    }

    async fn fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::CorrectlyFunded> {
        let fund_action = params.build_fund_action(deploy_event.location);
        self.beta_wallet.fund(fund_action).await?;

        let event = herc20::watch_for_funded(
            self.beta_wallet.connector.as_ref(),
            params,
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(event)
    }

    async fn refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Refunded> {
        let refund_action = params.build_refund_action(deploy_event.location);
        self.beta_wallet.refund(refund_action).await?;

        let refund_event = herc20::watch_for_refunded(
            self.beta_wallet.connector.as_ref(),
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(refund_event)
    }
}

impl<BW, DB> WalletBob<bitcoin::Wallet, BW, DB, hbit::PrivateDetailsRedeemer> {
    async fn redeem(
        &self,
        params: hbit::Params,
        fund_event: hbit::CorrectlyFunded,
        secret: Secret,
    ) -> anyhow::Result<hbit::Redeemed> {
        let redeem_action = params.build_redeem_action(
            &crate::SECP,
            fund_event.asset,
            fund_event.location,
            self.private_protocol_details.clone().transient_redeem_sk,
            self.private_protocol_details.clone().final_redeem_identity,
            secret,
        )?;
        let transaction = self.alpha_wallet.redeem(redeem_action).await?;
        let redeem_event = hbit::Redeemed {
            transaction,
            secret,
        };

        Ok(redeem_event)
    }
}

#[cfg(test)]
pub mod watch_only_actor {
    //! This module is only useful for integration tests, given that
    //! Nectar always executes a swap as Bob.

    use super::*;
    use comit::btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock};
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    pub struct WatchOnlyBob<AC, BC, DB> {
        pub alpha_connector: Arc<AC>,
        pub beta_connector: Arc<BC>,
        pub db: DB,
        pub secret_hash: SecretHash,
        pub start_of_swap: NaiveDateTime,
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> herc20::Deploy for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: db::Load<herc20::Deployed> + db::Save<herc20::Deployed>,
    {
        async fn deploy(
            &self,
            params: herc20::Params,
            beta_expiry: Timestamp,
        ) -> anyhow::Result<Next<herc20::Deployed>> {
            {
                if let Some(deploy_event) = self.db.load(0).await? {
                    return Ok(Next::Continue(deploy_event));
                }

                let beta_ledger_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
                if beta_expiry <= beta_ledger_time {
                    return Ok(Next::Abort);
                }

                let deploy_event = herc20::watch_for_deployed(
                    self.beta_connector.as_ref(),
                    params,
                    self.start_of_swap,
                )
                .await?;
                self.db.save(deploy_event.clone(), 0).await?;

                Ok(Next::Continue(deploy_event))
            }
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> herc20::Fund for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: db::Load<herc20::CorrectlyFunded> + db::Save<herc20::CorrectlyFunded>,
    {
        async fn fund(
            &self,
            params: herc20::Params,
            deploy_event: herc20::Deployed,
            beta_expiry: Timestamp,
        ) -> anyhow::Result<Next<herc20::CorrectlyFunded>> {
            {
                if let Some(fund_event) = self.db.load(0).await? {
                    return Ok(Next::Continue(fund_event));
                }

                let beta_ledger_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
                if beta_expiry <= beta_ledger_time {
                    return Ok(Next::Abort);
                }

                let fund_event = herc20::watch_for_funded(
                    self.beta_connector.as_ref(),
                    params,
                    self.start_of_swap,
                    deploy_event,
                )
                .await?;
                self.db.save(fund_event.clone(), 0).await?;

                Ok(Next::Continue(fund_event))
            }
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> hbit::RedeemAsBob for WatchOnlyBob<AC, BC, DB>
    where
        AC: LatestBlock<Block = bitcoin::Block>
            + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
    {
        async fn redeem(
            &self,
            params: &hbit::Params,
            fund_event: hbit::CorrectlyFunded,
            _secret: Secret,
        ) -> anyhow::Result<hbit::Redeemed> {
            let event = hbit::watch_for_redeemed(
                self.alpha_connector.as_ref(),
                &params,
                fund_event.location,
                self.start_of_swap,
            )
            .await?;

            Ok(event)
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB> herc20::Refund for WatchOnlyBob<AC, BC, DB>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
    {
        async fn refund(
            &self,
            _params: herc20::Params,
            deploy_event: herc20::Deployed,
        ) -> anyhow::Result<herc20::Refunded> {
            let event = herc20::watch_for_refunded(
                self.beta_connector.as_ref(),
                self.start_of_swap,
                deploy_event,
            )
            .await?;

            Ok(event)
        }
    }
}
