use crate::swap::{hbit, BetaLedgerTime};
use comit::{
    asset,
    bitcoin::median_time_past,
    btsieve::{bitcoin::BitcoindConnector, BlockByHash, LatestBlock},
    Secret, Timestamp,
};
use std::{sync::Arc, time::Duration};

pub use crate::bitcoin::Amount;
pub use ::bitcoin::{secp256k1::SecretKey, Address, Block, BlockHash, OutPoint, Transaction};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Arc<crate::bitcoin::Wallet>,
    pub connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
}

#[async_trait::async_trait]
impl hbit::ExecuteFund for Wallet {
    async fn execute_fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::Funded> {
        let action = params.shared.build_fund_action();

        let txid = self
            .inner
            .send_to_address(action.to, action.amount.into(), action.network)
            .await?;
        let transaction = self.inner.get_raw_transaction(txid).await?;

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
            .find(|(_, txout)| {
                txout.script_pubkey == params.shared.compute_address().script_pubkey()
            })
            .map(|(vout, _txout)| bitcoin::OutPoint { txid, vout });

        let location = location.ok_or_else(|| {
            anyhow::anyhow!("Fund transaction does not contain expected outpoint")
        })?;
        let asset = asset::Bitcoin::from_sat(transaction.output[location.vout as usize].value);

        Ok(hbit::Funded { asset, location })
    }
}

#[async_trait::async_trait]
impl hbit::ExecuteRedeem for Wallet {
    async fn execute_redeem(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
        secret: Secret,
    ) -> anyhow::Result<hbit::Redeemed> {
        let redeem_address = self.inner.new_address().await?;

        let action = params.shared.build_redeem_action(
            &crate::SECP,
            fund_event.asset,
            fund_event.location,
            params.transient_sk,
            redeem_address,
            secret,
        )?;
        let transaction = self.spend(action).await?;

        Ok(hbit::Redeemed {
            transaction,
            secret,
        })
    }
}

/// Trigger the refund path of the HTLC corresponding to the
/// `hbit::Params` and the `hbit::Funded` event passed, once it's
/// possible.
#[async_trait::async_trait]
impl hbit::ExecuteRefund for Wallet {
    async fn execute_refund(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
    ) -> anyhow::Result<hbit::Refunded> {
        loop {
            let bitcoin_time = comit::bitcoin::median_time_past(self.connector.as_ref()).await?;

            if bitcoin_time >= params.shared.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_address = self.inner.new_address().await?;

        let action = params.shared.build_refund_action(
            &crate::SECP,
            fund_event.asset,
            fund_event.location,
            params.transient_sk,
            refund_address,
        )?;
        let transaction = self.spend(action).await?;

        Ok(hbit::Refunded { transaction })
    }
}

impl Wallet {
    async fn spend(
        &self,
        action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<bitcoin::Transaction> {
        let _txid = self
            .inner
            .send_raw_transaction(action.transaction.clone(), action.network)
            .await?;

        Ok(action.transaction)
    }
}

#[async_trait::async_trait]
impl LatestBlock for Wallet {
    type Block = bitcoin::Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.as_ref().latest_block().await
    }
}

#[async_trait::async_trait]
impl BlockByHash for Wallet {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;
    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        self.connector.as_ref().block_by_hash(block_hash).await
    }
}

#[async_trait::async_trait]
impl BetaLedgerTime for BitcoindConnector {
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
        median_time_past(self).await
    }
}

#[async_trait::async_trait]
impl BetaLedgerTime for Wallet {
    async fn beta_ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.connector.as_ref().beta_ledger_time().await
    }
}
