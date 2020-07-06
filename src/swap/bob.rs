//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::{
    swap::{
        db, ethereum, BetaExpiry, BetaLedgerTime, Execute, {hbit, herc20},
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, SecretHash, Timestamp};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW, DB, AP, BP, E> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: DB,
    pub alpha_params: AP,
    pub beta_params: BP,
    pub private_protocol_details: E,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP, E> Execute<herc20::Deployed> for WalletBob<AW, BW, DB, AP, herc20::Params, E>
where
    AW: Send + Sync,
    BW: herc20::ExecuteDeploy + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
    E: Send + Sync,
{
    type Args = herc20::Params;

    async fn execute(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        self.beta_wallet.execute_deploy(params).await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP, E> Execute<herc20::CorrectlyFunded>
    for WalletBob<AW, BW, DB, AP, herc20::Params, E>
where
    AW: Send + Sync,
    BW: herc20::ExecuteFund + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
    E: Send + Sync,
{
    type Args = (herc20::Params, herc20::Deployed);

    async fn execute(
        &self,
        (params, deploy_event): (herc20::Params, herc20::Deployed),
    ) -> anyhow::Result<herc20::CorrectlyFunded> {
        self.beta_wallet
            .execute_fund(params, deploy_event, self.start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, BP> Execute<hbit::Redeemed>
    for WalletBob<AW, BW, DB, hbit::Params, BP, hbit::PrivateDetailsRedeemer>
where
    AW: hbit::ExecuteRedeem + Send + Sync,
    BW: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = (hbit::Params, hbit::CorrectlyFunded, Secret);

    async fn execute(
        &self,
        (params, fund_event, secret): (hbit::Params, hbit::CorrectlyFunded, Secret),
    ) -> anyhow::Result<hbit::Redeemed> {
        self.alpha_wallet
            .execute_redeem(
                params,
                fund_event,
                secret,
                self.private_protocol_details.transient_redeem_sk,
            )
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP, E> herc20::Refund for WalletBob<AW, BW, DB, AP, herc20::Params, E>
where
    AW: Send + Sync,
    BW: BetaLedgerTime + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
    E: Send + Sync,
{
    async fn refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Refunded> {
        loop {
            if self.beta_wallet.beta_ledger_time().await? >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_event = self.refund(params, deploy_event).await?;

        Ok(refund_event)
    }
}

impl<AW, DB, AP, BP, E> WalletBob<AW, ethereum::Wallet, DB, AP, BP, E> {
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

impl<AW, BW, DB, AP, E> BetaExpiry for WalletBob<AW, BW, DB, AP, herc20::Params, E> {
    fn beta_expiry(&self) -> Timestamp {
        self.beta_params.expiry
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP, BP, E> BetaLedgerTime for WalletBob<AW, BW, DB, AP, BP, E>
where
    AW: Send + Sync,
    BW: BetaLedgerTime + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
    BP: Send + Sync,
    E: Send + Sync,
{
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.beta_wallet.beta_ledger_time().await
    }
}

#[async_trait::async_trait]
impl<T, AW, BW, DB, AP, BP, E> db::Load<T> for WalletBob<AW, BW, DB, AP, BP, E>
where
    T: 'static,
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Load<T>,
    AP: Send + Sync + 'static,
    BP: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>> {
        self.db.load(swap_id).await
    }
}

#[async_trait::async_trait]
impl<T, AW, BW, DB, AP, BP, E> db::Save<T> for WalletBob<AW, BW, DB, AP, BP, E>
where
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Save<T>,
    AP: Send + Sync + 'static,
    BP: Send + Sync + 'static,
    E: Send + Sync + 'static,
    T: Send + 'static,
{
    async fn save(&self, event: T, swap_id: SwapId) -> anyhow::Result<()> {
        self.db.save(event, swap_id).await
    }
}

impl<AW, BW, DB, AP, BP, E> std::ops::Deref for WalletBob<AW, BW, DB, AP, BP, E> {
    type Target = SwapId;
    fn deref(&self) -> &Self::Target {
        &self.swap_id
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
    pub struct WatchOnlyBob<AC, BC, DB, AP, BP> {
        pub alpha_connector: Arc<AC>,
        pub beta_connector: Arc<BC>,
        pub db: DB,
        pub alpha_params: AP,
        pub beta_params: BP,
        pub secret_hash: SecretHash,
        pub start_of_swap: NaiveDateTime,
        pub swap_id: SwapId,
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, AP> Execute<herc20::Deployed> for WatchOnlyBob<AC, BC, DB, AP, herc20::Params>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
        AP: Send + Sync,
    {
        type Args = herc20::Params;

        async fn execute(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
            herc20::watch_for_deployed(self.beta_connector.as_ref(), params, self.start_of_swap)
                .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, AP> Execute<herc20::CorrectlyFunded>
        for WatchOnlyBob<AC, BC, DB, AP, herc20::Params>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
        AP: Send + Sync,
    {
        type Args = (herc20::Params, herc20::Deployed);

        async fn execute(
            &self,
            (params, deploy_event): (herc20::Params, herc20::Deployed),
        ) -> anyhow::Result<herc20::CorrectlyFunded> {
            herc20::watch_for_funded(
                self.beta_connector.as_ref(),
                params,
                self.start_of_swap,
                deploy_event,
            )
            .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, BP> Execute<hbit::Redeemed> for WatchOnlyBob<AC, BC, DB, hbit::Params, BP>
    where
        AC: LatestBlock<Block = bitcoin::Block>
            + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
        BC: Send + Sync,
        DB: Send + Sync,
        BP: Send + Sync,
    {
        type Args = (hbit::Params, hbit::CorrectlyFunded, Secret);

        async fn execute(
            &self,
            (params, fund_event, _): (hbit::Params, hbit::CorrectlyFunded, Secret),
        ) -> anyhow::Result<hbit::Redeemed> {
            hbit::watch_for_redeemed(
                self.alpha_connector.as_ref(),
                &params,
                fund_event.location,
                self.start_of_swap,
            )
            .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, AP> herc20::Refund for WatchOnlyBob<AC, BC, DB, AP, herc20::Params>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
        AP: Send + Sync,
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

    impl<AC, BC, DB, AP> BetaExpiry for WatchOnlyBob<AC, BC, DB, AP, herc20::Params> {
        fn beta_expiry(&self) -> Timestamp {
            self.beta_params.expiry
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, AP, BP> BetaLedgerTime for WatchOnlyBob<AC, BC, DB, AP, BP>
    where
        AC: Send + Sync,
        BC: BetaLedgerTime + Send + Sync,
        DB: Send + Sync,
        AP: Send + Sync,
        BP: Send + Sync,
    {
        async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
            self.beta_connector.as_ref().beta_ledger_time().await
        }
    }

    #[async_trait::async_trait]
    impl<T, AC, BC, DB, AP, BP> db::Load<T> for WatchOnlyBob<AC, BC, DB, AP, BP>
    where
        T: 'static,
        AC: Send + Sync + 'static,
        BC: Send + Sync + 'static,
        DB: db::Load<T>,
        AP: Send + Sync + 'static,
        BP: Send + Sync + 'static,
    {
        async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>> {
            self.db.load(swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<T, AC, BC, DB, AP, BP> db::Save<T> for WatchOnlyBob<AC, BC, DB, AP, BP>
    where
        T: Send + 'static,
        AC: Send + Sync + 'static,
        BC: Send + Sync + 'static,
        DB: db::Save<T>,
        AP: Send + Sync + 'static,
        BP: Send + Sync + 'static,
    {
        async fn save(&self, event: T, swap_id: SwapId) -> anyhow::Result<()> {
            self.db.save(event, swap_id).await
        }
    }

    impl<AC, BC, DB, AP, BP> std::ops::Deref for WatchOnlyBob<AC, BC, DB, AP, BP> {
        type Target = SwapId;
        fn deref(&self) -> &Self::Target {
            &self.swap_id
        }
    }
}
