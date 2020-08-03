//! Bob's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so this
//! component has to be prepared to execute actions using wallets.

use crate::{
    swap::{
        db, LedgerTime, {hbit, herc20},
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{Secret, SecretHash, Timestamp};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WalletBob<AW, BW, DB> {
    pub alpha_wallet: AW,
    pub beta_wallet: BW,
    pub db: Arc<DB>,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[async_trait::async_trait]
impl<AW, BW, DB> herc20::ExecuteDeploy for WalletBob<AW, BW, DB>
where
    AW: Send + Sync,
    BW: herc20::ExecuteDeploy + Send + Sync,
    DB: Send + Sync,
{
    async fn execute_deploy(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        self.beta_wallet.execute_deploy(params).await
    }
}

#[async_trait::async_trait]
impl<AW, BW, DB> herc20::ExecuteFund for WalletBob<AW, BW, DB>
where
    AW: Send + Sync,
    BW: herc20::ExecuteFund + Send + Sync,
    DB: Send + Sync,
{
    async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Funded> {
        self.beta_wallet
            .execute_fund(params, deploy_event, start_of_swap)
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
impl<AW, BW, DB> herc20::ExecuteRefund for WalletBob<AW, BW, DB>
where
    AW: Send + Sync,
    BW: herc20::ExecuteRefund + Send + Sync,
    DB: Send + Sync,
{
    async fn execute_refund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<herc20::Refunded> {
        self.beta_wallet
            .execute_refund(params, deploy_event, start_of_swap)
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
impl<AW, BW, DB> hbit::ExecuteRedeem for WalletBob<AW, BW, DB>
where
    AW: hbit::ExecuteRedeem + Send + Sync,
    BW: Send + Sync,
    DB: Send + Sync,
{
    async fn execute_redeem(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
        secret: Secret,
    ) -> anyhow::Result<comit::hbit::Redeemed> {
        self.alpha_wallet
            .execute_redeem(params, fund_event, secret)
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

#[async_trait::async_trait]
impl<AW, BW, DB> LedgerTime for WalletBob<AW, BW, DB>
where
    AW: Send + Sync,
    BW: LedgerTime + Send + Sync,
    DB: Send + Sync,
{
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.beta_wallet.ledger_time().await
    }
}

impl<E, AW, BW, DB> db::Load<E> for WalletBob<AW, BW, DB>
where
    E: 'static,
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Load<E>,
{
    fn load(&self, swap_id: SwapId) -> anyhow::Result<Option<E>> {
        self.db.load(swap_id)
    }
}

#[async_trait::async_trait]
impl<E, AW, BW, DB> db::Save<E> for WalletBob<AW, BW, DB>
where
    E: Send + 'static,
    AW: Send + Sync + 'static,
    BW: Send + Sync + 'static,
    DB: db::Save<E>,
{
    async fn save(&self, event: E, swap_id: SwapId) -> anyhow::Result<()> {
        self.db.save(event, swap_id).await
    }
}
