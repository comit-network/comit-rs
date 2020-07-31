//! This module is only useful for integration tests, given that
//! Nectar never executes a swap as Alice.

use crate::{
    swap::{db, hbit, herc20, AsSwapId, BetaExpiry, BetaLedgerTime},
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, Timestamp};
use std::sync::Arc;

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

#[async_trait::async_trait]
impl<AW, BW, DB, BP> hbit::ExecuteFund for WalletAlice<AW, BW, DB, hbit::Params, BP>
where
    AW: hbit::ExecuteFund + Send + Sync,
    BW: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    async fn execute_fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::Funded> {
        self.alpha_wallet.execute_fund(params).await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, AP> herc20::ExecuteRedeem for WalletAlice<AW, BW, DB, AP, herc20::Params>
where
    AW: Send + Sync,
    BW: herc20::ExecuteRedeem + Send + Sync,
    DB: Send + Sync,
    AP: Send + Sync,
{
    async fn execute_redeem(
        &self,
        params: herc20::Params,
        secret: Secret,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Redeemed> {
        self.beta_wallet
            .execute_redeem(params, secret, deploy_event, start_of_swap)
            .await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB, BP> hbit::ExecuteRefund for WalletAlice<AW, BW, DB, hbit::Params, BP>
where
    AW: hbit::ExecuteRefund + Send + Sync,
    BW: Send + Sync,
    DB: Send + Sync,
    BP: Send + Sync,
{
    async fn execute_refund(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
    ) -> anyhow::Result<comit::hbit::Refunded> {
        self.alpha_wallet.execute_refund(params, fund_event).await
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

impl<E, AW, BW, DB, AP, BP> db::Load<E> for WalletAlice<AW, BW, DB, AP, BP>
where
    E: 'static,
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Load<E>,
    AP: Send + Sync + 'static,
    BP: Send + Sync + 'static,
{
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<E>> {
        self.db.load(swap_id)
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

impl<AW, BW, DB, AP, BP> AsSwapId for WalletAlice<AW, BW, DB, AP, BP> {
    fn as_swap_id(&self) -> SwapId {
        self.swap_id
    }
}
