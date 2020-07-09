//! Alice's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so our local
//! representation of the other party, Alice, is a component that
//! watches the two blockchains involved in the swap.

use crate::{
    swap::{bitcoin, db, ethereum, hbit, herc20, BetaExpiry, BetaLedgerTime, Execute},
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
    pub db: Arc<DB>,
    pub alpha_params: AP,
    pub beta_params: BP,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[allow(clippy::unit_arg)]
#[async_trait::async_trait]
impl<AC, BC, DB, BP> Execute<herc20::Deployed> for WatchOnlyAlice<AC, BC, DB, herc20::Params, BP>
where
    AC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = ();

    async fn execute(&self, (): Self::Args) -> anyhow::Result<herc20::Deployed> {
        herc20::watch_for_deployed(
            self.alpha_connector.as_ref(),
            self.alpha_params.clone(),
            self.start_of_swap,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB, BP> Execute<herc20::Funded> for WatchOnlyAlice<AC, BC, DB, herc20::Params, BP>
where
    AC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = herc20::Deployed;

    async fn execute(&self, deploy_event: herc20::Deployed) -> anyhow::Result<herc20::Funded> {
        herc20::watch_for_funded(
            self.alpha_connector.as_ref(),
            self.alpha_params.clone(),
            self.start_of_swap,
            deploy_event,
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

/// Nectar does not care if the taker refunds, so we do not need to
/// look for the taker's refund event the blockchain.
///
/// Therefore, this implementation is effectively a no-op.
#[async_trait::async_trait]
impl<AC, BC, DB, BP> Execute<herc20::Refunded> for WatchOnlyAlice<AC, BC, DB, herc20::Params, BP>
where
    AC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = herc20::Deployed;

    async fn execute(&self, _deploy_event: herc20::Deployed) -> anyhow::Result<herc20::Refunded> {
        Ok(herc20::Refunded {
            transaction: ethereum::Transaction::default(),
        })
    }
}

#[allow(clippy::unit_arg)]
#[async_trait::async_trait]
impl<AC, BC, DB, BP> Execute<hbit::Funded> for WatchOnlyAlice<AC, BC, DB, hbit::SharedParams, BP>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = ();

    async fn execute(&self, (): Self::Args) -> anyhow::Result<hbit::Funded> {
        hbit::watch_for_funded(
            self.alpha_connector.as_ref(),
            &self.alpha_params,
            self.start_of_swap,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB, AP> Execute<hbit::Redeemed> for WatchOnlyAlice<AC, BC, DB, AP, hbit::SharedParams>
where
    AC: Send + Sync,
    BC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    DB: Send + Sync,
    AP: Send + Sync,
{
    type Args = hbit::Funded;

    async fn execute(&self, fund_event: hbit::Funded) -> anyhow::Result<hbit::Redeemed> {
        hbit::watch_for_redeemed(
            self.beta_connector.as_ref(),
            &self.beta_params,
            fund_event.location,
            self.start_of_swap,
        )
        .await
    }
}

/// Nectar does not care if the taker refunds, so we do not need to
/// look for the taker's refund event the blockchain.
///
/// Therefore, this implementation is effectively a no-op.
#[async_trait::async_trait]
impl<AC, BC, DB, BP> Execute<hbit::Refunded> for WatchOnlyAlice<AC, BC, DB, hbit::SharedParams, BP>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    type Args = hbit::Funded;

    async fn execute(&self, _fund_event: hbit::Funded) -> anyhow::Result<hbit::Refunded> {
        Ok(hbit::Refunded {
            transaction: bitcoin::Transaction {
                version: 1,
                lock_time: 0,
                input: Vec::new(),
                output: Vec::new(),
            },
        })
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
impl<E, AC, BC, DB, AP, BP> db::Load<E> for WatchOnlyAlice<AC, BC, DB, AP, BP>
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
impl<E, AC, BC, DB, AP, BP> db::Save<E> for WatchOnlyAlice<AC, BC, DB, AP, BP>
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

    #[derive(Clone, Debug)]
    pub struct WalletAlice<AW, BW, DB, AP, BP> {
        pub alpha_wallet: AW,
        pub beta_wallet: BW,
        pub db: Arc<DB>,
        pub alpha_params: AP,
        pub beta_params: BP,
        pub secret: Secret,
        pub start_of_swap: NaiveDateTime,
        pub swap_id: SwapId,
    }

    #[allow(clippy::unit_arg)]
    #[async_trait::async_trait]
    impl<AW, BW, DB, BP> Execute<hbit::Funded> for WalletAlice<AW, BW, DB, hbit::Params, BP>
    where
        AW: hbit::ExecuteFund + Send + Sync,
        BW: Send + Sync,
        DB: Send + Sync,
        BP: Send + Sync,
    {
        type Args = ();

        async fn execute(&self, (): Self::Args) -> anyhow::Result<hbit::Funded> {
            self.alpha_wallet.execute_fund(&self.alpha_params).await
        }
    }

    #[async_trait::async_trait]
    impl<AW, BW, DB, AP> Execute<herc20::Redeemed> for WalletAlice<AW, BW, DB, AP, herc20::Params>
    where
        AW: Send + Sync,
        BW: herc20::ExecuteRedeem + Send + Sync,
        DB: Send + Sync,
        AP: Send + Sync,
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
    impl<BW, DB, BP> Execute<hbit::Refunded> for WalletAlice<bitcoin::Wallet, BW, DB, hbit::Params, BP>
    where
        BW: Send + Sync,
        DB: Send + Sync,
        BP: Send + Sync,
    {
        type Args = hbit::Funded;

        async fn execute(&self, fund_event: hbit::Funded) -> anyhow::Result<hbit::Refunded> {
            loop {
                let bitcoin_time =
                    comit::bitcoin::median_time_past(self.alpha_wallet.connector.as_ref()).await?;

                if bitcoin_time >= self.alpha_params.shared.expiry {
                    break;
                }

                tokio::time::delay_for(Duration::from_secs(1)).await;
            }

            let refund_event = self.refund(fund_event).await?;

            Ok(refund_event)
        }
    }

    impl<BW, DB, BP> WalletAlice<bitcoin::Wallet, BW, DB, hbit::Params, BP> {
        async fn refund(&self, fund_event: hbit::Funded) -> anyhow::Result<hbit::Refunded> {
            let refund_address = self.alpha_wallet.inner.new_address().await?;
            let refund_action = self.alpha_params.shared.build_refund_action(
                &crate::SECP,
                fund_event.asset,
                fund_event.location,
                self.alpha_params.transient_sk,
                refund_address,
            )?;
            let transaction = self.alpha_wallet.refund(refund_action).await?;
            let refund_event = hbit::Refunded { transaction };

            Ok(refund_event)
        }
    }

    impl<AW, BW, DB, AP> BetaExpiry for WalletAlice<AW, BW, DB, AP, herc20::Params> {
        fn beta_expiry(&self) -> Timestamp {
            self.beta_params.expiry
        }
    }

    #[async_trait::async_trait]
    impl<AW, BW, DB, AP, BP> BetaLedgerTime for WalletAlice<AW, BW, DB, AP, BP>
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
    impl<E, AW, BW, DB, AP, BP> db::Load<E> for WalletAlice<AW, BW, DB, AP, BP>
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
    impl<E, AW, BW, DB, AP, BP> db::Save<E> for WalletAlice<AW, BW, DB, AP, BP>
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

    impl<AW, BW, DB, AP, BP> std::ops::Deref for WalletAlice<AW, BW, DB, AP, BP> {
        type Target = SwapId;
        fn deref(&self) -> &Self::Target {
            &self.swap_id
        }
    }
}
