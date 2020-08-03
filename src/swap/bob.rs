//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::{
    swap::{
        action::{poll_beta_has_expired, try_do_it_once},
        bitcoin,
        db::Database,
        ethereum, LedgerTime, {hbit, herc20},
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, SecretHash, Timestamp};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: Arc<Database>,
    pub swap_id: SwapId,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub beta_expiry: Timestamp,
}

#[async_trait::async_trait]
impl<AW> herc20::ExecuteDeploy for WalletBob<AW, ethereum::Wallet>
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
impl<AW> herc20::ExecuteFund for WalletBob<AW, ethereum::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Funded> {
        let action = self
            .beta_wallet
            .execute_fund(params, deploy_event, start_of_swap);
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

// #[async_trait::async_trait]
// impl<AW, BW, DB, BP> Execute<herc20::Redeemed> for WalletBob<AW, BW, DB, herc20::Params, BP>
// where
//     AW: herc20::ExecuteRedeem + Send + Sync,
//     BW: Send + Sync,
//     DB: Send + Sync,
//     BP: Send + Sync,
// {
//     type Args = (herc20::Deployed, Secret);

//     async fn execute(
//         &self,
//         (deploy_event, secret): (herc20::Deployed, Secret),
//     ) -> anyhow::Result<herc20::Redeemed> {
//         self.alpha_wallet
//             .execute_redeem(
//                 self.alpha_params.clone(),
//                 secret,
//                 deploy_event,
//                 self.start_of_swap,
//             )
//             .await
//     }
// }

#[async_trait::async_trait]
impl<AW> herc20::ExecuteRefund for WalletBob<AW, ethereum::Wallet>
where
    AW: Send + Sync,
{
    async fn execute_refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Refunded> {
        let action = self
            .beta_wallet
            .execute_refund(params, deploy_event, start_of_swap);

        try_do_it_once(
            self.db.as_ref(),
            self.swap_id,
            action,
            futures::future::pending(),
        )
        .await
    }
}

// #[allow(clippy::unit_arg)]
// #[async_trait::async_trait]
// impl<AW, BW, DB, AP> Execute<hbit::Funded> for WalletBob<AW, BW, DB, AP, hbit::Params>
// where
//     AW: Send + Sync,
//     BW: hbit::ExecuteFund + Send + Sync,
//     DB: Send + Sync,
//     AP: Send + Sync,
// {
//     type Args = ();

//     async fn execute(&self, (): Self::Args) -> anyhow::Result<hbit::Funded> {
//         self.beta_wallet.execute_fund(&self.beta_params).await
//     }
// }

#[async_trait::async_trait]
impl<BW> hbit::ExecuteRedeem for WalletBob<bitcoin::Wallet, BW>
where
    BW: LedgerTime + Send + Sync,
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

// #[async_trait::async_trait]
// impl<AW, BW, DB, AP> Execute<hbit::Refunded> for WalletBob<AW, BW, DB, AP, hbit::Params>
// where
//     AW: Send + Sync,
//     BW: hbit::ExecuteRefund + Send + Sync,
//     DB: Send + Sync,
//     AP: Send + Sync,
// {
//     type Args = hbit::Funded;

//     async fn execute(&self, fund_event: Self::Args) -> anyhow::Result<hbit::Refunded> {
//         self.beta_wallet
//             .execute_refund(self.beta_params, fund_event)
//             .await
//     }
// }
