use crate::swap::hbit;
use comit::btsieve::{BlockByHash, LatestBlock};
use std::sync::Arc;

pub use ::bitcoin::{Address, Block, BlockHash, OutPoint};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: crate::bitcoin_wallet::Wallet,
    pub connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
}

impl Wallet {
    pub async fn fund(&self, action: hbit::SendToAddress) -> anyhow::Result<bitcoin::Transaction> {
        let txid = self
            .inner
            .send_to_address(action.to, action.amount.into(), action.network)
            .await?;
        let transaction = self.inner.get_raw_transaction(txid).await?;

        Ok(transaction)
    }

    pub async fn redeem(
        &self,
        action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<bitcoin::Transaction> {
        self.spend(action).await
    }

    pub async fn refund(
        &self,
        action: hbit::BroadcastSignedTransaction,
    ) -> anyhow::Result<bitcoin::Transaction> {
        self.spend(action).await
    }

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
    type Block = Block;
    type BlockHash = BlockHash;
    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        self.connector.as_ref().block_by_hash(block_hash).await
    }
}
