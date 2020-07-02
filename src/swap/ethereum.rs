use crate::swap::herc20;
use comit::btsieve::LatestBlock;
use comit::Timestamp;
use std::sync::Arc;

pub use comit::ethereum::{Address, Block, ChainId, Hash};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: crate::ethereum_wallet::Wallet,
    pub connector: Arc<comit::btsieve::ethereum::Web3Connector>,
}

impl Wallet {
    pub async fn deploy(&self, action: herc20::DeployContract) -> anyhow::Result<herc20::Deployed> {
        let transaction_hash = self.inner.deploy_contract(action).await?;
        let transaction = self.inner.get_transaction_by_hash(transaction_hash).await?;
        let receipt = self.inner.get_transaction_receipt(transaction_hash).await?;

        let location = receipt
            .contract_address
            .ok_or_else(|| anyhow::anyhow!("Contract address missing from receipt"))?;

        Ok(herc20::Deployed {
            transaction,
            location,
        })
    }

    pub async fn fund(&self, action: herc20::CallContract) -> anyhow::Result<()> {
        let _ = self.inner.call_contract(action).await?;

        Ok(())
    }

    pub async fn redeem(&self, action: herc20::CallContract) -> anyhow::Result<()> {
        let _ = self.inner.call_contract(action).await?;

        Ok(())
    }

    pub async fn refund(&self, action: herc20::CallContract) -> anyhow::Result<()> {
        let _ = self.inner.call_contract(action).await?;

        Ok(())
    }
}

pub async fn ethereum_latest_time<C>(connector: &C) -> anyhow::Result<Timestamp>
where
    C: LatestBlock<Block = Block>,
{
    let timestamp = connector.latest_block().await?.timestamp.into();

    Ok(timestamp)
}

#[async_trait::async_trait]
impl LatestBlock for Wallet {
    type Block = Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.latest_block().await
    }
}
