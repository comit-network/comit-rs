use crate::swap::Next;
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
    async fn deploy(
        &self,
        params: Params,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<Deployed>>;
}

#[async_trait::async_trait]
pub trait ExecuteDeploy {
    async fn execute_deploy(&self, params: Params) -> anyhow::Result<Deployed>;
}

#[async_trait::async_trait]
pub trait Fund {
    async fn fund(
        &self,
        params: Params,
        deploy_event: Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<CorrectlyFunded>>;
}

#[async_trait::async_trait]
pub trait ExecuteFund {
    async fn execute_fund(
        &self,
        params: Params,
        deploy_event: Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<CorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait RedeemAsAlice {
    async fn redeem(
        &self,
        params: Params,
        deploy_event: Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<Redeemed>>;
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
    async fn refund(&self, params: Params, deploy_event: Deployed) -> anyhow::Result<Refunded>;
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

#[cfg(test)]
pub fn params(
    secret_hash: SecretHash,
    chain_id: crate::swap::ethereum::ChainId,
    redeem_identity: identity::Ethereum,
    refund_identity: identity::Ethereum,
    token_contract: crate::swap::ethereum::Address,
    expiry: Timestamp,
) -> Params {
    let quantity = asset::ethereum::FromWei::from_wei(1_000_000_000u64);
    let asset = asset::Erc20::new(token_contract, quantity);

    Params {
        asset,
        redeem_identity,
        refund_identity,
        expiry,
        chain_id,
        secret_hash,
    }
}
