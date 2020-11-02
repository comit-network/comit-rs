use crate::{bitcoin, swap::hbit};
use comit::{
    btsieve::{BlockByHash, LatestBlock},
    Secret,
};
use std::sync::Arc;

pub use crate::bitcoin::Amount;
pub use ::bitcoin::{secp256k1::SecretKey, Address, Block, BlockHash, OutPoint, Transaction};
use comit::actions::bitcoin::BroadcastSignedTransaction;

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Arc<crate::bitcoin::Wallet>,
    pub fee: bitcoin::Fee,
    pub connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
}

impl Wallet {
    pub async fn execute_fund(&self, params: &hbit::Params) -> anyhow::Result<hbit::Funded> {
        let action = params.build_fund_action();

        let kbyte_fee_rate = self.fee.kvbyte_rate().await?;

        let location = self
            .inner
            .fund_htlc(action.to, action.amount, action.network, kbyte_fee_rate)
            .await?;

        let txid = location.txid;

        tracing::info!("signed hbit fund transaction {}", txid);

        Ok(hbit::Funded {
            asset: action.amount,
            location,
        })
    }

    pub async fn execute_redeem(
        &self,
        params: hbit::Params,
        fund_event: hbit::Funded,
        secret: Secret,
    ) -> anyhow::Result<hbit::Redeemed> {
        let vbyte_rate = self.fee.vbyte_rate().await?;

        let action =
            params.build_redeem_action(&crate::SECP, fund_event.location, secret, vbyte_rate)?;
        let transaction = self.spend(action).await?;
        let txid = transaction.txid();

        tracing::info!("signed hbit redeem transaction {}", txid);

        Ok(hbit::Redeemed {
            transaction: txid,
            secret,
        })
    }

    pub async fn spend(
        &self,
        action: BroadcastSignedTransaction,
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
