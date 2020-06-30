//! Wrapper module around COMIT lib's Herc20 module.

use chrono::NaiveDateTime;
pub use comit::{
    actions::ethereum::*,
    asset,
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    ethereum::{Block, ChainId, Hash},
    herc20::*,
    identity, transaction, Secret, SecretHash, Timestamp,
};

#[async_trait::async_trait]
pub trait Deploy {
    async fn deploy(&self, params: &Params) -> anyhow::Result<Deployed>;
}

#[async_trait::async_trait]
pub trait Fund {
    async fn fund(&self, params: Params, deploy_event: Deployed)
        -> anyhow::Result<CorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait RedeemAsAlice {
    async fn redeem(&self, params: &Params, deploy_event: Deployed) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait RedeemAsBob {
    async fn redeem(
        &self,
        params: &Params,
        deploy_event: Deployed,
        secret: Secret,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait Refund {
    async fn refund(&self, params: &Params, deploy_event: Deployed) -> anyhow::Result<Refunded>;
}

#[derive(Debug, Clone)]
pub struct CorrectlyFunded {
    pub transaction: transaction::Ethereum,
    pub asset: asset::Erc20,
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: Params,
    start_of_swap: NaiveDateTime,
    deployed: Deployed,
) -> anyhow::Result<CorrectlyFunded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    match comit::herc20::watch_for_funded(connector, params, start_of_swap, deployed).await? {
        comit::herc20::Funded::Correctly { transaction, asset } => {
            Ok(CorrectlyFunded { transaction, asset })
        }
        comit::herc20::Funded::Incorrectly { .. } => {
            anyhow::bail!("Ethereum HTLC incorrectly funded")
        }
    }
}
