//! Wrapper module around COMIT lib's Hbit module.

use bitcoin::{secp256k1::SecretKey, Address, Block, BlockHash};
use chrono::NaiveDateTime;
use comit::asset;

pub use comit::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
    btsieve::{BlockByHash, LatestBlock},
    hbit::*,
    htlc_location, transaction, Secret, SecretHash,
};

#[derive(Clone, Debug)]
pub struct PrivateDetailsFunder {
    pub transient_refund_sk: SecretKey,
    pub final_refund_identity: Address,
}

#[derive(Clone, Debug)]
pub struct PrivateDetailsRedeemer {
    pub transient_redeem_sk: SecretKey,
    pub final_redeem_identity: Address,
}

#[async_trait::async_trait]
pub trait Fund {
    async fn fund(&self, params: &Params) -> anyhow::Result<CorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait RedeemAsAlice {
    async fn redeem(
        &self,
        params: &Params,
        fund_event: CorrectlyFunded,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait RedeemAsBob {
    async fn redeem(
        &self,
        params: &Params,
        fund_event: CorrectlyFunded,
        secret: Secret,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait Refund {
    async fn refund(
        &self,
        params: &Params,
        fund_event: CorrectlyFunded,
    ) -> anyhow::Result<Refunded>;
}

#[derive(Debug, Clone, Copy)]
pub struct CorrectlyFunded {
    pub asset: asset::Bitcoin,
    pub location: htlc_location::Bitcoin,
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: &Params,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<CorrectlyFunded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    match comit::hbit::watch_for_funded(connector, params, start_of_swap).await? {
        comit::hbit::Funded::Correctly {
            asset, location, ..
        } => Ok(CorrectlyFunded { asset, location }),
        comit::hbit::Funded::Incorrectly { .. } => anyhow::bail!("Bitcoin HTLC incorrectly funded"),
    }
}
