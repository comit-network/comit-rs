//! Alice's perspective of the swap.
//!
//! In Nectar we always take the role of Bob in the swap, so our local
//! representation of the other party, Alice, is a component that
//! watches the two blockchains involved in the swap.

use crate::swap::{
    ethereum::{self, ethereum_latest_time},
    hbit, herc20, Decision, Next,
};
use chrono::NaiveDateTime;
use comit::{
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    SecretHash, Timestamp,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WatchOnlyAlice<AC, BC> {
    pub alpha_connector: Arc<AC>,
    pub beta_connector: Arc<BC>,
    pub secret_hash: SecretHash,
    pub start_of_swap: NaiveDateTime,
}

#[async_trait::async_trait]
impl<AC, BC> hbit::Fund for WatchOnlyAlice<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: LatestBlock<Block = ethereum::Block>,
{
    async fn fund(
        &self,
        params: &hbit::Params,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<hbit::CorrectlyFunded>> {
        if let Some(fund_event) = hbit::watch_for_funded_in_the_past(
            self.alpha_connector.as_ref(),
            params,
            self.start_of_swap,
        )
        .await?
        {
            return Ok(Next::Continue(fund_event));
        }

        let beta_ledger_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
        if beta_expiry <= beta_ledger_time {
            return Ok(Next::Abort);
        }

        let fund_event =
            hbit::watch_for_funded(self.alpha_connector.as_ref(), &params, self.start_of_swap)
                .await?;

        Ok(Next::Continue(fund_event))
    }
}

#[async_trait::async_trait]
impl<AC, BC> herc20::RedeemAsAlice for WatchOnlyAlice<AC, BC>
where
    AC: Send + Sync,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn redeem(
        &self,
        _params: &herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Redeemed> {
        let event = herc20::watch_for_redeemed(
            self.beta_connector.as_ref(),
            self.start_of_swap,
            deploy_event,
        )
        .await?;

        Ok(event)
    }
}

#[async_trait::async_trait]
impl<AC, BC> hbit::Refund for WatchOnlyAlice<AC, BC>
where
    AC: LatestBlock<Block = bitcoin::Block>
        + BlockByHash<Block = bitcoin::Block, BlockHash = bitcoin::BlockHash>,
    BC: Send + Sync,
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

#[async_trait::async_trait]
impl<AC, BC> herc20::DecideOnRedeem for WatchOnlyAlice<AC, BC>
where
    AC: Send + Sync,
    BC: LatestBlock<Block = ethereum::Block>
        + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
        + ReceiptByHash,
{
    async fn decide_on_redeem(
        &self,
        herc20_params: herc20::Params,
        deploy_event: herc20::Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<crate::swap::Decision<herc20::Redeemed>> {
        {
            if let Some(redeem_event) = herc20::watch_for_redeemed_in_the_past(
                self.beta_connector.as_ref(),
                herc20_params,
                self.start_of_swap,
                deploy_event,
            )
            .await?
            {
                return Ok(Decision::Skip(redeem_event));
            }

            let beta_ledger_time = ethereum_latest_time(self.beta_connector.as_ref()).await?;
            // TODO: Apply a buffer depending on the blocktime and how
            // safe we want to be

            if beta_expiry > beta_ledger_time {
                Ok(Decision::Act)
            } else {
                Ok(Decision::Stop)
            }
        }
    }
}

#[cfg(test)]
pub mod wallet_actor {
    //! This module is only useful for integration tests, given that
    //! Nectar never executes a swap as Alice.

    use super::*;
    use crate::swap::bitcoin;
    use anyhow::Context;
    use comit::{asset, Secret};
    use std::time::Duration;

    #[derive(Clone, Copy, Debug)]
    pub struct WalletAlice<AW, BW, E> {
        pub alpha_wallet: AW,
        pub beta_wallet: BW,
        pub private_protocol_details: E,
        pub secret: Secret,
        pub start_of_swap: NaiveDateTime,
    }

    #[async_trait::async_trait]
    impl<BW> hbit::Fund for WalletAlice<bitcoin::Wallet, BW, hbit::PrivateDetailsFunder>
    where
        BW: LatestBlock<Block = ethereum::Block>,
    {
        async fn fund(
            &self,
            params: &hbit::Params,
            beta_expiry: Timestamp,
        ) -> anyhow::Result<Next<hbit::CorrectlyFunded>> {
            if let Some(fund_event) =
                hbit::watch_for_funded_in_the_past(&self.alpha_wallet, params, self.start_of_swap)
                    .await?
            {
                return Ok(Next::Continue(fund_event));
            }

            let beta_ledger_time = ethereum_latest_time(&self.beta_wallet).await?;
            if beta_expiry <= beta_ledger_time {
                return Ok(Next::Abort);
            }

            let fund_event = self.fund(&params).await?;
            Ok(Next::Continue(fund_event))
        }
    }

    impl<BW> WalletAlice<bitcoin::Wallet, BW, hbit::PrivateDetailsFunder> {
        async fn fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::CorrectlyFunded> {
            let fund_action = params.build_fund_action();
            let transaction = self
                .alpha_wallet
                .fund(fund_action)
                .await
                .context("failed to fund bitcoin HTLC")?;

            let txid = transaction.txid();
            // TODO: This code is copied straight from COMIT lib. We
            // should find a way of not having to duplicate this logic
            let location = transaction
                .output
                .iter()
                .enumerate()
                .map(|(index, txout)| {
                    // Casting a usize to u32 can lead to truncation on 64bit platforms
                    // However, bitcoin limits the number of inputs to u32 anyway, so this
                    // is not a problem for us.
                    #[allow(clippy::cast_possible_truncation)]
                    (index as u32, txout)
                })
                .find(|(_, txout)| txout.script_pubkey == params.compute_address().script_pubkey())
                .map(|(vout, _txout)| bitcoin::OutPoint { txid, vout });

            let location = location.ok_or_else(|| {
                anyhow::anyhow!("Fund transaction does not contain expected outpoint")
            })?;
            let asset = asset::Bitcoin::from_sat(transaction.output[location.vout as usize].value);

            Ok(hbit::CorrectlyFunded { asset, location })
        }
    }

    #[async_trait::async_trait]
    impl herc20::RedeemAsAlice
        for WalletAlice<bitcoin::Wallet, ethereum::Wallet, hbit::PrivateDetailsFunder>
    {
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
    impl hbit::Refund for WalletAlice<bitcoin::Wallet, ethereum::Wallet, hbit::PrivateDetailsFunder> {
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

            let refund_action = params.build_refund_action(
                &crate::SECP,
                fund_event.asset,
                fund_event.location,
                self.private_protocol_details.transient_refund_sk,
                self.private_protocol_details.final_refund_identity.clone(),
            )?;
            let transaction = self.alpha_wallet.refund(refund_action).await?;
            let refunded = hbit::Refunded { transaction };

            Ok(refunded)
        }
    }

    #[async_trait::async_trait]
    impl<AW, BW, E> herc20::DecideOnRedeem for WalletAlice<AW, BW, E>
    where
        AW: Send + Sync,
        BW: LatestBlock<Block = ethereum::Block>
            + BlockByHash<Block = ethereum::Block, BlockHash = ethereum::Hash>
            + ReceiptByHash,
        E: Send + Sync,
    {
        async fn decide_on_redeem(
            &self,
            herc20_params: herc20::Params,
            deploy_event: herc20::Deployed,
            beta_expiry: Timestamp,
        ) -> anyhow::Result<crate::swap::Decision<herc20::Redeemed>> {
            {
                if let Some(redeem_event) = herc20::watch_for_redeemed_in_the_past(
                    &self.beta_wallet,
                    herc20_params,
                    self.start_of_swap,
                    deploy_event,
                )
                .await?
                {
                    return Ok(Decision::Skip(redeem_event));
                }

                let beta_ledger_time = ethereum_latest_time(&self.beta_wallet).await?;
                // TODO: Apply a buffer depending on the blocktime and how
                // safe we want to be

                if beta_expiry > beta_ledger_time {
                    Ok(Decision::Act)
                } else {
                    Ok(Decision::Stop)
                }
            }
        }
    }
}
