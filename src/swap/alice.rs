//! Alice's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so our local
//! representation of the other party, Alice, is a component that
//! watches the two blockchains involved in the swap.

use crate::{
    swap::{
        db, ethereum, hbit, herc20, BlockchainTime, CheckMemory, Execute, Next, Remember,
        ShouldAbort,
    },
    SwapId,
};
use chrono::NaiveDateTime;
use comit::{
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    SecretHash, Timestamp,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WatchOnlyAlice<AC, BC, DB> {
    pub alpha_connector: Arc<AC>,
    pub beta_connector: Arc<BC>,
    pub db: DB,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
    pub swap_id: SwapId,
}

#[async_trait::async_trait]
impl<T, AC, BC, DB> CheckMemory<T> for WatchOnlyAlice<AC, BC, DB>
where
    AC: Send + Sync,
    BC: Send + Sync,
    DB: db::Load<T>,
    T: 'static,
{
    async fn check_memory(&self) -> anyhow::Result<Option<T>> {
        self.db.load(self.swap_id).await
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB> ShouldAbort for WatchOnlyAlice<AC, BC, DB>
where
    AC: Send + Sync,
    BC: BlockchainTime + Send + Sync,
    DB: Send + Sync,
{
    async fn should_abort(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
        let beta_blockchain_time = self.beta_connector.as_ref().blockchain_time().await?;

        Ok(beta_expiry <= beta_blockchain_time)
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB> Execute<hbit::CorrectlyFunded> for WatchOnlyAlice<AC, BC, DB>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
    DB: Send + Sync,
{
    type Args = hbit::Params;

    async fn execute(&self, params: hbit::Params) -> anyhow::Result<hbit::CorrectlyFunded> {
        hbit::watch_for_funded(self.alpha_connector.as_ref(), &params, self.start_of_swap).await
    }
}

#[async_trait::async_trait]
impl<T, AC, BC, DB> Remember<T> for WatchOnlyAlice<AC, BC, DB>
where
    AC: Send + Sync,
    BC: Send + Sync,
    DB: db::Save<T>,
    T: Send + 'static,
{
    async fn remember(&self, event: T) -> anyhow::Result<()> {
        self.db.save(event, self.swap_id).await
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB> herc20::RedeemAsAlice for WatchOnlyAlice<AC, BC, DB>
where
    AC: Send + Sync,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
    DB: db::Load<herc20::Redeemed> + db::Save<herc20::Redeemed>,
{
    async fn redeem(
        &self,
        _params: herc20::Params,
        deploy_event: herc20::Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<herc20::Redeemed>> {
        {
            if let Some(redeem_event) = self.db.load(self.swap_id).await? {
                return Ok(Next::Continue(redeem_event));
            }

            if beta_expiry <= self.beta_connector.as_ref().blockchain_time().await? {
                return Ok(Next::Abort);
            }

            let redeem_event = herc20::watch_for_redeemed(
                self.beta_connector.as_ref(),
                self.start_of_swap,
                deploy_event,
            )
            .await?;
            self.db.save(redeem_event.clone(), self.swap_id).await?;

            Ok(Next::Continue(redeem_event))
        }
    }
}

#[async_trait::async_trait]
impl<AC, BC, DB> hbit::Refund for WatchOnlyAlice<AC, BC, DB>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
    DB: Send + Sync,
{
    async fn refund(
        &self,
        params: &hbit::Params,
        fund_event: hbit::CorrectlyFunded,
    ) -> anyhow::Result<hbit::Refunded> {
        let event = hbit::watch_for_refunded(
            self.alpha_connector.as_ref(),
            params,
            fund_event.location,
            self.start_of_swap,
        )
        .await?;

        Ok(event)
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
    pub struct WalletAlice<AW, BW, DB, E> {
        pub alpha_wallet: AW,
        pub beta_wallet: BW,
        pub db: DB,
        pub private_protocol_details: E,
        pub secret: Secret,
        pub start_of_swap: NaiveDateTime,
        pub swap_id: SwapId,
    }

    #[async_trait::async_trait]
    impl<AW, BW, DB, E> Execute<hbit::CorrectlyFunded> for WalletAlice<AW, BW, DB, E>
    where
        AW: hbit::ExecuteFund + Send + Sync,
        BW: Send + Sync,
        DB: Send + Sync,
        E: Send + Sync,
    {
        type Args = hbit::Params;

        async fn execute(&self, params: hbit::Params) -> anyhow::Result<hbit::CorrectlyFunded> {
            self.alpha_wallet.execute_fund(&params).await
        }
    }

    #[async_trait::async_trait]
    impl<AW, DB, E> herc20::RedeemAsAlice for WalletAlice<AW, ethereum::Wallet, DB, E>
    where
        AW: Send + Sync,
        DB: db::Load<herc20::Redeemed> + db::Save<herc20::Redeemed>,
        E: Send + Sync,
    {
        async fn redeem(
            &self,
            params: herc20::Params,
            deploy_event: herc20::Deployed,
            beta_expiry: Timestamp,
        ) -> anyhow::Result<Next<herc20::Redeemed>> {
            {
                if let Some(redeem_event) = self.db.load(self.swap_id).await? {
                    return Ok(Next::Continue(redeem_event));
                }

                if beta_expiry <= self.beta_wallet.blockchain_time().await? {
                    return Ok(Next::Abort);
                }

                let redeem_event = self.redeem(&params, deploy_event).await?;
                self.db.save(redeem_event.clone(), self.swap_id).await?;

                Ok(Next::Continue(redeem_event))
            }
        }
    }

    #[async_trait::async_trait]
    impl<BW, DB> hbit::Refund for WalletAlice<bitcoin::Wallet, BW, DB, hbit::PrivateDetailsFunder>
    where
        BW: Send + Sync,
        DB: Send + Sync,
    {
        async fn refund(
            &self,
            params: &hbit::Params,
            fund_event: hbit::CorrectlyFunded,
        ) -> anyhow::Result<hbit::Refunded> {
            loop {
                let bitcoin_time =
                    comit::bitcoin::median_time_past(self.alpha_wallet.connector.as_ref()).await?;

                if bitcoin_time >= params.expiry {
                    break;
                }

                tokio::time::delay_for(Duration::from_secs(1)).await;
            }

            let refund_event = self.refund(params, fund_event).await?;

            Ok(refund_event)
        }
    }

    impl<BW, DB> WalletAlice<bitcoin::Wallet, BW, DB, hbit::PrivateDetailsFunder> {
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

    impl<AW, DB, E> WalletAlice<AW, ethereum::Wallet, DB, E> {
        async fn redeem(
            &self,
            params: &herc20::Params,
            deploy_event: herc20::Deployed,
        ) -> anyhow::Result<herc20::Redeemed> {
            let redeem_action = params.build_redeem_action(deploy_event.location, self.secret);
            self.beta_wallet.redeem(redeem_action).await?;

            let event = herc20::watch_for_redeemed(
                self.beta_wallet.connector.as_ref(),
                self.start_of_swap,
                deploy_event,
            )
            .await?;

            Ok(event)
        }
    }

    #[async_trait::async_trait]
    impl<T, AW, BW, DB, E> CheckMemory<T> for WalletAlice<AW, BW, DB, E>
    where
        AW: Send + Sync,
        BW: Send + Sync,
        DB: db::Load<T>,
        E: Send + Sync,
        T: 'static,
    {
        async fn check_memory(&self) -> anyhow::Result<Option<T>> {
            self.db.load(self.swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<T, AW, BW, DB, E> Remember<T> for WalletAlice<AW, BW, DB, E>
    where
        AW: Send + Sync,
        BW: Send + Sync,
        DB: db::Save<T>,
        E: Send + Sync,
        T: Send + 'static,
    {
        async fn remember(&self, event: T) -> anyhow::Result<()> {
            self.db.save(event, self.swap_id).await
        }
    }

    #[async_trait::async_trait]
    impl<AW, BW, DB, E> ShouldAbort for WalletAlice<AW, BW, DB, E>
    where
        AW: Send + Sync,
        BW: BlockchainTime + Send + Sync,
        DB: Send + Sync,
        E: Send + Sync,
    {
        async fn should_abort(&self, beta_expiry: Timestamp) -> anyhow::Result<bool> {
            let beta_blockchain_time = self.beta_wallet.blockchain_time().await?;

            Ok(beta_expiry <= beta_blockchain_time)
        }
    }
}
