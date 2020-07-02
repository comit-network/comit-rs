//! Wrapper module around COMIT lib's Herc20 module.

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
pub trait Fund {
    async fn fund(
        &self,
        params: Params,
        deploy_event: Deployed,
        beta_expiry: Timestamp,
    ) -> anyhow::Result<Next<CorrectlyFunded>>;
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

pub async fn watch_for_deployed_in_the_past<C>(
    _connector: &C,
    _params: Params,
    _start_of_swap: NaiveDateTime,
) -> anyhow::Result<Option<Deployed>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    todo!()
}

pub async fn watch_for_funded_in_the_past<C>(
    _connector: &C,
    _params: Params,
    _start_of_swap: NaiveDateTime,
    _deployed: Deployed,
) -> anyhow::Result<Option<CorrectlyFunded>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    todo!()
}

pub async fn watch_for_redeemed_in_the_past<C>(
    _connector: &C,
    _params: Params,
    _start_of_swap: NaiveDateTime,
    _deployed: Deployed,
) -> anyhow::Result<Option<Redeemed>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    todo!()
}
