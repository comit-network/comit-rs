use crate::{
    bitcoin,
    database::Database,
    swap::{hbit, LedgerTime},
};
use comit::{
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
    pub fee: bitcoin::Fee,
    pub connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
    pub db: Arc<Database>,
}

#[async_trait::async_trait]
impl hbit::ExecuteFund for Wallet {
    async fn execute_fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::Funded> {
        let action = params.build_fund_action();

        let kbyte_fee_rate = self.fee.kvbyte_rate().await?;

        let location = self
            .inner
            .fund_htlc(action.to, action.amount, action.network, kbyte_fee_rate)
            .await?;
        let asset = action.amount;

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

        let vbyte_rate = self.fee.vbyte_rate().await?;

        let action = params.build_redeem_action(
            &crate::SECP,
            fund_event.asset,
            fund_event.location,
            self.db.load_secret_key(params.redeem_identity).await?,
            redeem_address,
            secret,
            vbyte_rate,
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

            if bitcoin_time >= params.expiry {
                break;
            }

            tokio::time::delay_for(Duration::from_secs(1)).await;
        }

        let refund_address = self.inner.new_address().await?;

        let vbyte_rate = self.fee.vbyte_rate().await?;

        let action = params.build_refund_action(
            &crate::SECP,
            fund_event.asset,
            fund_event.location,
            self.db.load_secret_key(params.refund_identity).await?,
            refund_address,
            vbyte_rate,
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
impl LedgerTime for BitcoindConnector {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        median_time_past(self).await
    }
}

#[async_trait::async_trait]
impl LedgerTime for Wallet {
    async fn ledger_time(&self) -> anyhow::Result<Timestamp> {
        self.connector.as_ref().ledger_time().await
    }
}
