//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::{
    swap::{
        action::try_do_it_once, bitcoin, ethereum, hbit, herc20, poll_beta_has_expired, Database,
    },
    SwapId,
};
use chrono::{DateTime, Utc};
use comit::{Secret, SecretHash, Timestamp};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Bob<AW, BW> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: Arc<Database>,
    pub swap_id: SwapId,
    pub secret_hash: SecretHash,
    pub utc_start_of_swap: DateTime<Utc>,
    pub beta_expiry: Timestamp,
}

#[async_trait::async_trait]
impl<AW> herc20::ExecuteDeploy for Bob<AW, ethereum::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_deploy(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let action = self.beta_wallet.execute_deploy(params);
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
impl<AW> herc20::ExecuteFund for Bob<AW, ethereum::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        utc_start_of_swap: DateTime<Utc>,
    ) -> anyhow::Result<herc20::Funded> {
        let action = self
            .beta_wallet
            .execute_fund(params, deploy_event, utc_start_of_swap);
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
impl<BW> herc20::ExecuteRedeem for Bob<ethereum::Wallet, BW>
where
    BW: Send + Sync,
{
    async fn execute_redeem(
        &self,
        params: comit::herc20::Params,
        secret: Secret,
        deploy_event: comit::herc20::Deployed,
        utc_start_of_swap: DateTime<Utc>,
    ) -> anyhow::Result<comit::herc20::Redeemed> {
        let action =
            self.alpha_wallet
                .execute_redeem(params, secret, deploy_event, utc_start_of_swap);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            futures::future::pending(),
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AW> herc20::ExecuteRefund for Bob<AW, ethereum::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        utc_start_of_swap: DateTime<Utc>,
    ) -> anyhow::Result<herc20::Refunded> {
        let action = self
            .beta_wallet
            .execute_refund(params, deploy_event, utc_start_of_swap);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            futures::future::pending(),
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AW> hbit::ExecuteFund for Bob<AW, bitcoin::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::Funded> {
        let action = self.beta_wallet.execute_fund(params);
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
impl<BW> hbit::ExecuteRedeem for Bob<bitcoin::Wallet, BW>
where
    BW: Send + Sync,
{
    async fn execute_redeem(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
        secret: Secret,
    ) -> anyhow::Result<comit::hbit::Redeemed> {
        let action = self.alpha_wallet.execute_redeem(params, fund_event, secret);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            futures::future::pending(),
        )
        .await
    }
}

#[async_trait::async_trait]
impl<AW> hbit::ExecuteRefund for Bob<AW, bitcoin::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_refund(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
    ) -> anyhow::Result<comit::hbit::Refunded> {
        let action = self.beta_wallet.execute_refund(params, fund_event);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            futures::future::pending(),
        )
        .await
    }
}
