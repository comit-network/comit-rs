//! Alice's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so our local
//! representation of the other party, Alice, is a component that
//! watches the two blockchains involved in the swap.

use crate::{
    swap::{db, ethereum, hbit, herc20, BetaExpiry, BetaLedgerTime, Execute},
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    SecretHash, Timestamp,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WatchOnlyAlice<AC, BC, DB, AP, BP> {
    pub alpha_connector: Arc<AC>,
    pub beta_connector: Arc<BC>,
    pub db: DB,
    pub alpha_params: AP,
    pub beta_params: BP,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[allow(clippy::unit_arg)]
#[async_trait::async_trait]
impl<AC, BC, DB, BP> Execute<hbit::CorrectlyFunded> for WatchOnlyAlice<AC, BC, DB, hbit::Params, BP>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = ();

    async fn execute(&self, (): ()) -> anyhow::Result<hbit::CorrectlyFunded> {
        hbit::watch_for_funded(
            self.alpha_connector.as_ref(),
            &self.alpha_params,
            self.start_of_swap,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB, AP> Execute<herc20::Redeemed> for WatchOnlyAlice<AC, BC, DB, AP, herc20::Params>
where
    AC: Send + Sync,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = herc20::Deployed;

    async fn execute(&self, deploy_event: herc20::Deployed) -> anyhow::Result<herc20::Redeemed> {
        herc20::watch_for_redeemed(
            self.beta_connector.as_ref(),
            self.start_of_swap,
            deploy_event,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB, BP> hbit::Refund for WatchOnlyAlice<AC, BC, DB, hbit::Params, BP>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    async fn refund(&self, fund_event: hbit::CorrectlyFunded) -> anyhow::Result<hbit::Refunded> {
        let event = hbit::watch_for_refunded(
            self.alpha_connector.as_ref(),
            &self.alpha_params,
            fund_event.location,
            self.start_of_swap,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB, AP> BetaExpiry for WatchOnlyAlice<AC, BC, DB, AP, herc20::Params> {
    fn beta_expiry(&self) -> Timestamp {
        self.beta_params.expiry
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB, AP, BP> BetaLedgerTime for WatchOnlyAlice<AC, BC, DB, AP, BP>
where
    AC: Send + Sync,
    BC: BetaLedgerTime + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
    BP: Send + Sync,
{
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.beta_connector.beta_ledger_time().await
    }
}

#[async_trait::async_trait]
impl<T, AC, BC, DB, AP, BP> db::Load<T> for WatchOnlyAlice<AC, BC, DB, AP, BP>
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
impl<T, AC, BC, DB, AP, BP> db::Save<T> for WatchOnlyAlice<AC, BC, DB, AP, BP>
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

impl<AC, BC, DB, AP, BP> std::ops::Deref for WatchOnlyAlice<AC, BC, DB, AP, BP> {
    type Target = SwapId;
    fn deref(&self) -> &Self::Target {
        &self.swap_id
    }
}

#[cfg(test)]
pub mod wallet_actor {
    //! This module is only useful for integration tests, given that
    //! Nectar never executes a swap as Alice.

    use super::*;
    use crate::swap::bitcoin;
    use comit::Secret;
    use std::time::Duration;

    #[derive(Clone, Copy, Debug)]
    pub struct WalletAlice<AW, BW, DB, AP, BP, E> {
        pub alpha_wallet: AW,
        pub beta_wallet: BW,
        pub db: DB,
        pub alpha_params: AP,
        pub beta_params: BP,
        pub private_protocol_details: E,
        pub secret: Secret,
        pub start_of_swap: NaiveDateTime,
        pub swap_id: SwapId,
    }

    #[allow(clippy::unit_arg)]
    #[async_trait::async_trait]
    impl<AW, BW, DB, BP, E> Execute<hbit::CorrectlyFunded>
        for WalletAlice<AW, BW, DB, hbit::Params, BP, E>
    where
        AW: hbit::ExecuteFund + Send + Sync,
        BW: Send + Sync,
        DB: Send + Sync,
        BP: Send + Sync,
        E: Send + Sync,
    {
        type Args = ();

        async fn execute(&self, (): ()) -> anyhow::Result<hbit::CorrectlyFunded> {
            self.alpha_wallet.execute_fund(&self.alpha_params).await
        }
    }

    #[async_trait::async_trait]
    impl<AW, BW, DB, AP, E> Execute<herc20::Redeemed> for WalletAlice<AW, BW, DB, AP, herc20::Params, E>
    where
        AW: Send + Sync,
        BW: herc20::ExecuteRedeem + Send + Sync,
        DB: Send + Sync,
        AP: Send + Sync,
        E: Send + Sync,
    {
        type Args = herc20::Deployed;

        async fn execute(
            &self,
            deploy_event: herc20::Deployed,
        ) -> anyhow::Result<herc20::Redeemed> {
            self.beta_wallet
                .execute_redeem(
                    self.beta_params.clone(),
                    self.secret,
                    deploy_event,
                    self.start_of_swap,
                )
                .await
        }
    }

    #[async_trait::async_trait]
    impl<BW, DB, BP> hbit::Refund
        for WalletAlice<bitcoin::Wallet, BW, DB, hbit::Params, BP, hbit::PrivateDetailsFunder>
    where
        BW: Send + Sync,
        DB: Send + Sync,
        BP: Send + Sync,
    {
        async fn refund(
            &self,
            fund_event: hbit::CorrectlyFunded,
        ) -> anyhow::Result<hbit::Refunded> {
            loop {
                let bitcoin_time =
                    comit::bitcoin::median_time_past(self.alpha_wallet.connector.as_ref()).await?;

                if bitcoin_time >= self.alpha_params.expiry {
                    break;
                }

                tokio::time::delay_for(Duration::from_secs(1)).await;
            }

            let refund_event = self.refund(&self.alpha_params, fund_event).await?;

            Ok(refund_event)
        }
    }

    impl<BW, DB, BP>
        WalletAlice<bitcoin::Wallet, BW, DB, hbit::Params, BP, hbit::PrivateDetailsFunder>
    {
        async fn refund(
            &self,
            params: &hbit::Params,
            fund_event: hbit::CorrectlyFunded,
        ) -> anyhow::Result<hbit::Refunded> {
            let refund_action = params.build_refund_action(
                &crate::SECP,
                fund_event.asset,
                fund_event.location,
                self.private_protocol_details.transient_refund_sk,
                self.private_protocol_details.final_refund_identity.clone(),
            )?;
            let transaction = self.alpha_wallet.refund(refund_action).await?;
            let refund_event = hbit::Refunded { transaction };

            Ok(refund_event)
        }
    }

    impl<AW, BW, DB, AP, E> BetaExpiry for WalletAlice<AW, BW, DB, AP, herc20::Params, E> {
        fn beta_expiry(&self) -> Timestamp {
            self.beta_params.expiry
        }
    }

    #[async_trait::async_trait]
    impl<AW, BW, DB, AP, BP, E> BetaLedgerTime for WalletAlice<AW, BW, DB, AP, BP, E>
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
    impl<T, AW, BW, DB, AP, BP, E> db::Load<T> for WalletAlice<AW, BW, DB, AP, BP, E>
    where
        AW: Send + Sync + 'static,
        BW: Send + Sync + 'static,
        DB: db::Load<T>,
        AP: Send + Sync + 'static,
        BP: Send + Sync + 'static,
        E: Send + Sync + 'static,
        T: 'static,
    {
        async fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<T>> {
            self.db.load(swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<T, AW, BW, DB, AP, BP, E> db::Save<T> for WalletAlice<AW, BW, DB, AP, BP, E>
    where
        AW: Send + Sync + 'static,
        BW: Send + Sync + 'static,
        DB: db::Save<T>,
        E: Send + Sync + 'static,
        AP: Send + Sync + 'static,
        BP: Send + Sync + 'static,
        T: Send + 'static,
    {
        async fn save(&self, event: T, swap_id: SwapId) -> anyhow::Result<()> {
            self.db.save(event, swap_id).await
        }
    }

    impl<AW, BW, DB, AP, BP, E> std::ops::Deref for WalletAlice<AW, BW, DB, AP, BP, E> {
        type Target = SwapId;
        fn deref(&self) -> &Self::Target {
            &self.swap_id
        }
    }
}
