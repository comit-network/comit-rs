//! This module is only useful for integration tests, given that
//! Nectar never executes a swap as Alice.

use crate::{
    swap::{
        action::{poll_beta_has_expired, try_do_it_once},
        bitcoin,
        db::Database,
        ethereum, hbit, herc20, LedgerTime,
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, Timestamp};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WalletAlice<AW, BW> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: Arc<Database>,
    pub swap_id: SwapId,
    pub secret: Secret,
    pub start_of_swap: NaiveDateTime,
    pub beta_expiry: Timestamp,
}

#[async_trait::async_trait]
impl<BW> hbit::ExecuteFund for WalletAlice<bitcoin::Wallet, BW>
where
    BW: LedgerTime + Send + Sync,
{
    async fn execute_fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::Funded> {
        let action = self.alpha_wallet.execute_fund(params);
        let poll_beta_has_expired = poll_beta_has_expired(&self.beta_wallet, self.beta_expiry);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            poll_beta_has_expired,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AW> herc20::ExecuteRedeem for WalletAlice<AW, ethereum::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_redeem(
        &self,
        params: herc20::Params,
        secret: Secret,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Redeemed> {
        let action = self
            .beta_wallet
            .execute_redeem(params, secret, deploy_event, start_of_swap);
        let poll_beta_has_expired = poll_beta_has_expired(&self.beta_wallet, self.beta_expiry);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            poll_beta_has_expired,
        )
        .await
    }
}

#[async_trait::async_trait]
impl<BW> hbit::ExecuteRefund for WalletAlice<bitcoin::Wallet, BW>
where
    BW: LedgerTime + Send + Sync,
{
    async fn execute_refund(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
    ) -> anyhow::Result<comit::hbit::Refunded> {
        let action = self.alpha_wallet.execute_refund(params, fund_event);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            futures::future::pending(),
        )
        .await
    }
}
