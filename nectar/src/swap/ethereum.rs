use crate::swap::herc20;
use comit::btsieve::LatestBlock;
use std::sync::Arc;

use comit::swap::actions::{CallContract, DeployContract};
pub use comit::{
    ethereum::{Address, Block, ChainId, Hash, Transaction},
    Secret,
};

#[derive(Debug, Clone)]
pub struct Wallet {
    pub inner: Arc<crate::ethereum::Wallet>,
    pub connector: Arc<comit::btsieve::ethereum::Web3Connector>,
    pub gas_price: crate::ethereum::GasPrice,
}

impl Wallet {
    pub async fn execute_deploy(&self, action: DeployContract) -> anyhow::Result<herc20::Deployed> {
        let gas_price = self.gas_price.gas_price().await?;
        let (tx_hash, contract_address) = self.inner.deploy_contract(action, gas_price).await?;

        tracing::info!("signed herc20 deploy transaction {}", tx_hash);

        Ok(herc20::Deployed {
            transaction: tx_hash,
            location: contract_address,
        })
    }

    pub async fn execute_fund(&self, action: CallContract) -> anyhow::Result<herc20::Funded> {
        let gas_price = self.gas_price.gas_price().await?;
        let tx_hash = self.inner.call_contract(action, gas_price).await?;

        tracing::info!("signed herc20 fund transaction {}", tx_hash);

        Ok(herc20::Funded {
            transaction: tx_hash,
        })
    }

    pub async fn execute_redeem(
        &self,
        action: CallContract,
        secret: Secret, /* Receiving the secret here is a bit of a hack but otherwise, we have
                         * to get it out of the action again which is even more cumbersome. */
    ) -> anyhow::Result<herc20::Redeemed> {
        let gas_price = self.gas_price.gas_price().await?;
        let tx_hash = self.inner.call_contract(action, gas_price).await?;

        tracing::info!("signed herc20 redeem transaction {}", tx_hash);

        Ok(herc20::Redeemed {
            transaction: tx_hash,
            secret,
        })
    }
}

#[async_trait::async_trait]
impl LatestBlock for Wallet {
    type Block = Block;
    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        self.connector.latest_block().await
    }
}
