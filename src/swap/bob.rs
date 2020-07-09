//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::{
    swap::{
        db, BetaExpiry, BetaLedgerTime, Execute, {hbit, herc20},
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, SecretHash, Timestamp};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW, DB, AP, BP> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: Arc<DB>,
    pub alpha_params: AP,
    pub beta_params: BP,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[allow(clippy::unit_arg)]
#[async_trait::async_trait]
impl<AW, BW, DB, AP> Execute<herc20::Deployed> for WalletBob<AW, BW, DB, AP, herc20::Params>
where
    AW: Send + Sync,
    BW: herc20::ExecuteDeploy + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = ();

    async fn execute(&self, (): Self::Args) -> anyhow::Result<herc20::Deployed> {
        self.beta_wallet
            .execute_deploy(self.beta_params.clone())
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP> Execute<herc20::Funded> for WalletBob<AW, BW, DB, AP, herc20::Params>
where
    AW: Send + Sync,
    BW: herc20::ExecuteFund + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = herc20::Deployed;

    async fn execute(&self, deploy_event: herc20::Deployed) -> anyhow::Result<herc20::Funded> {
        self.beta_wallet
            .execute_fund(self.beta_params.clone(), deploy_event, self.start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, BP> Execute<herc20::Redeemed> for WalletBob<AW, BW, DB, herc20::Params, BP>
where
    AW: herc20::ExecuteRedeem + Send + Sync,
    BW: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = (herc20::Deployed, Secret);

    async fn execute(
        &self,
        (deploy_event, secret): (herc20::Deployed, Secret),
    ) -> anyhow::Result<herc20::Redeemed> {
        self.alpha_wallet
            .execute_redeem(
                self.alpha_params.clone(),
                secret,
                deploy_event,
                self.start_of_swap,
            )
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP> Execute<herc20::Refunded> for WalletBob<AW, BW, DB, AP, herc20::Params>
where
    AW: Send + Sync,
    BW: herc20::ExecuteRefund + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = herc20::Deployed;

    async fn execute(&self, deploy_event: herc20::Deployed) -> anyhow::Result<herc20::Refunded> {
        self.beta_wallet
            .execute_refund(self.beta_params.clone(), deploy_event, self.start_of_swap)
            .await
    }
}

#[allow(clippy::unit_arg)]
#[async_trait::async_trait]
impl<AW, BW, DB, AP> Execute<hbit::Funded> for WalletBob<AW, BW, DB, AP, hbit::Params>
where
    AW: Send + Sync,
    BW: hbit::ExecuteFund + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = ();

    async fn execute(&self, (): Self::Args) -> anyhow::Result<hbit::Funded> {
        self.beta_wallet.execute_fund(&self.beta_params).await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, BP> Execute<hbit::Redeemed> for WalletBob<AW, BW, DB, hbit::Params, BP>
where
    AW: hbit::ExecuteRedeem + Send + Sync,
    BW: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = (hbit::Funded, Secret);

    async fn execute(
        &self,
        (fund_event, secret): (hbit::Funded, Secret),
    ) -> anyhow::Result<hbit::Redeemed> {
        self.alpha_wallet
            .execute_redeem(self.alpha_params, fund_event, secret)
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP> Execute<hbit::Refunded> for WalletBob<AW, BW, DB, AP, hbit::Params>
where
    AW: Send + Sync,
    BW: hbit::ExecuteRefund + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = hbit::Funded;

    async fn execute(&self, fund_event: Self::Args) -> anyhow::Result<hbit::Refunded> {
        self.beta_wallet
            .execute_refund(self.beta_params, fund_event)
            .await
    }
}

impl<AW, BW, DB, AP> BetaExpiry for WalletBob<AW, BW, DB, AP, herc20::Params> {
    fn beta_expiry(&self) -> Timestamp {
        self.beta_params.expiry
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP, BP> BetaLedgerTime for WalletBob<AW, BW, DB, AP, BP>
where
    AW: Send + Sync,
    BW: BetaLedgerTime + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
    BP: Send + Sync,
{
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.beta_wallet.beta_ledger_time().await
    }
}

#[async_trait::async_trait]
impl<E, AW, BW, DB, AP, BP> db::Load<E> for WalletBob<AW, BW, DB, AP, BP>
where
    E: 'static,
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Load<E>,
    AP: Send + Sync + 'static,
    BP: Send + Sync + 'static,
{
    async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<E>> {
        self.db.load(swap_id).await
    }
}

#[async_trait::async_trait]
impl<E, AW, BW, DB, AP, BP> db::Save<E> for WalletBob<AW, BW, DB, AP, BP>
where
    E: Send + 'static,
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Save<E>,
    AP: Send + Sync + 'static,
    BP: Send + Sync + 'static,
{
    async fn save(&self, event: E, swap_id: SwapId) -> anyhow::Result<()> {
        self.db.save(event, swap_id).await
    }
}

impl<AW, BW, DB, AP, BP> std::ops::Deref for WalletBob<AW, BW, DB, AP, BP> {
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
    use crate::swap::{bitcoin, ethereum};
    use comit::btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock};
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    pub struct WatchOnlyBob<AC, BC, DB, AP, BP> {
        pub alpha_connector: Arc<AC>,
        pub beta_connector: Arc<BC>,
        pub db: Arc<DB>,
        pub alpha_params: AP,
        pub beta_params: BP,
        pub secret_hash: SecretHash,
        pub start_of_swap: NaiveDateTime,
        pub swap_id: SwapId,
    }

    #[allow(clippy::unit_arg)]
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
        type Args = ();

        async fn execute(&self, (): Self::Args) -> anyhow::Result<herc20::Deployed> {
            herc20::watch_for_deployed(
                self.beta_connector.as_ref(),
                self.beta_params.clone(),
                self.start_of_swap,
            )
            .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, AP> Execute<herc20::Funded> for WatchOnlyBob<AC, BC, DB, AP, herc20::Params>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
        AP: Send + Sync,
    {
        type Args = herc20::Deployed;

        async fn execute(&self, deploy_event: herc20::Deployed) -> anyhow::Result<herc20::Funded> {
            herc20::watch_for_funded(
                self.beta_connector.as_ref(),
                self.beta_params.clone(),
                self.start_of_swap,
                deploy_event,
            )
            .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, BP> Execute<hbit::Redeemed> for WatchOnlyBob<AC, BC, DB, hbit::SharedParams, BP>
    where
        AC: LatestBlock<Block = bitcoin::Block>
            + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
        BC: Send + Sync,
        DB: Send + Sync,
        BP: Send + Sync,
    {
        type Args = (hbit::Funded, Secret);

        async fn execute(
            &self,
            (fund_event, _): (hbit::Funded, Secret),
        ) -> anyhow::Result<hbit::Redeemed> {
            hbit::watch_for_redeemed(
                self.alpha_connector.as_ref(),
                &self.alpha_params,
                fund_event.location,
                self.start_of_swap,
            )
            .await
        }
    }

    #[async_trait::async_trait]
    impl<AC, BC, DB, AP> Execute<herc20::Refunded> for WatchOnlyBob<AC, BC, DB, AP, herc20::Params>
    where
        AC: Send + Sync,
        BC: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        DB: Send + Sync,
        AP: Send + Sync,
    {
        type Args = herc20::Deployed;

        async fn execute(
            &self,
            deploy_event: herc20::Deployed,
        ) -> anyhow::Result<herc20::Refunded> {
            herc20::watch_for_refunded(
                self.beta_connector.as_ref(),
                self.start_of_swap,
                deploy_event,
            )
            .await
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
    impl<E, AC, BC, DB, AP, BP> db::Load<E> for WatchOnlyBob<AC, BC, DB, AP, BP>
    where
        E: 'static,
        AC: Send + Sync + 'static,
        BC: Send + Sync + 'static,
        DB: db::Load<E>,
        AP: Send + Sync + 'static,
        BP: Send + Sync + 'static,
    {
        async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<E>> {
            self.db.load(swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<E, AC, BC, DB, AP, BP> db::Save<E> for WatchOnlyBob<AC, BC, DB, AP, BP>
    where
        E: Send + 'static,
        AC: Send + Sync + 'static,
        BC: Send + Sync + 'static,
        DB: db::Save<E>,
        AP: Send + Sync + 'static,
        BP: Send + Sync + 'static,
    {
        async fn save(&self, event: E, swap_id: SwapId) -> anyhow::Result<()> {
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
