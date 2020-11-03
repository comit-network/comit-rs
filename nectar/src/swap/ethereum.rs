use crate::swap::herc20;
use comit::btsieve::LatestBlock;
use std::sync::Arc;

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
    pub async fn execute_deploy(&self, params: herc20::Params) -> anyhow::Result<herc20::Deployed> {
        let action = params.build_deploy_action();
        let gas_price = self.gas_price.gas_price().await?;
        let (tx_hash, contract_address) = self.inner.deploy_contract(action, gas_price).await?;

        tracing::info!("signed herc20 deploy transaction {}", tx_hash);

        Ok(herc20::Deployed {
            transaction: tx_hash,
            location: contract_address,
        })
    }

    pub async fn execute_fund(
        &self,
        params: herc20::Params,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Funded> {
        let action = params.build_fund_action(deploy_event.location);
        let gas_price = self.gas_price.gas_price().await?;
        let tx_hash = self.inner.call_contract(action, gas_price).await?;

        tracing::info!("signed herc20 fund transaction {}", tx_hash);

        Ok(herc20::Funded {
            transaction: tx_hash,
        })
    }

    pub async fn execute_redeem(
        &self,
        params: herc20::Params,
        secret: Secret,
        deploy_event: herc20::Deployed,
    ) -> anyhow::Result<herc20::Redeemed> {
        let action = params.build_redeem_action(deploy_event.location, secret);
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
