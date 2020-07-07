use bitcoin::{secp256k1::SecretKey, Block, BlockHash};
use chrono::NaiveDateTime;
use comit::asset;

pub use comit::{
    actions::bitcoin::{BroadcastSignedTransaction, SendToAddress},
    btsieve::{BlockByHash, LatestBlock},
    hbit::*,
    htlc_location, transaction, Secret, SecretHash, Timestamp,
};

pub type SharedParams = comit::hbit::Params;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub shared: SharedParams,
    pub transient_sk: SecretKey,
}

impl Params {
    pub fn new(shared: SharedParams, transient_sk: SecretKey) -> Self {
        Self {
            shared,
            transient_sk,
        }
    }
}

#[async_trait::async_trait]
pub trait ExecuteFund {
    async fn execute_fund(&self, params: &Params) -> anyhow::Result<Funded>;
}

#[async_trait::async_trait]
pub trait ExecuteRedeem {
    async fn execute_redeem(
        &self,
        params: Params,
        fund_event: Funded,
        secret: Secret,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait ExecuteRefund {
    async fn execute_refund(&self, params: Params, fund_event: Funded) -> anyhow::Result<Refunded>;
}

#[derive(Debug, Clone, Copy)]
pub struct Funded {
    pub asset: asset::Bitcoin,
    pub location: htlc_location::Bitcoin,
}

pub async fn watch_for_funded<C>(
    connector: &C,
    params: &SharedParams,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Funded>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = BlockHash>,
{
    match comit::hbit::watch_for_funded(connector, &params, start_of_swap).await? {
        comit::hbit::Funded::Correctly {
            asset, location, ..
        } => Ok(Funded { asset, location }),
        comit::hbit::Funded::Incorrectly { .. } => anyhow::bail!("Bitcoin HTLC incorrectly funded"),
    }
}
