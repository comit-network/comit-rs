use crate::{bitcoin, swap::hbit};
use anyhow::Result;
use comit::{
    btsieve::{BlockByHash, LatestBlock},
    swap::actions::{SendToAddress, SpendOutput},
    Secret,
};
use std::sync::Arc;

pub use crate::bitcoin::Amount;
pub use ::bitcoin::{secp256k1::SecretKey, Address, Block, BlockHash, OutPoint, Transaction};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Arc<crate::bitcoin::Wallet>,
    pub fee: bitcoin::Fee,
    pub connector: Arc<comit::btsieve::bitcoin::BitcoindConnector>,
}

impl Wallet {
    pub async fn execute_fund(&self, action: SendToAddress) -> Result<hbit::Funded> {
        let kbyte_fee_rate = self.fee.kvbyte_rate().await?;

        let location = self
            .inner
            .fund_htlc(action.to, action.amount, action.network, kbyte_fee_rate)
            .await?;

        let txid = location.txid;

        tracing::info!("signed hbit fund transaction {}", txid);

        Ok(hbit::Funded { location })
    }

    pub async fn execute_redeem(
        &self,
        action: SpendOutput,
        secret: Secret, /* Receiving the secret here is a bit of a hack but otherwise, we have
                         * to get it out of the action again which is even more cumbersome. */
    ) -> Result<hbit::Redeemed> {
        let vbyte_rate = self.fee.vbyte_rate().await?;
        let network = action.network;
        let transaction = action.sign(&crate::SECP, vbyte_rate)?;

        let txid = self
            .inner
            .send_raw_transaction(transaction, network)
            .await?;

        tracing::info!("signed hbit redeem transaction {}", txid);

        Ok(hbit::Redeemed {
            transaction: txid,
            secret,
        })
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
