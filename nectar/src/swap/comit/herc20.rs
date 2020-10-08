use anyhow::Result;
use comit::ethereum;
use thiserror::Error;
use time::OffsetDateTime;

pub use comit::{
    actions::ethereum::*,
    asset,
    btsieve::{ethereum::ReceiptByHash, BlockByHash, LatestBlock},
    ethereum::{Block, ChainId, Hash},
    herc20::{
        watch_for_deployed, watch_for_funded, watch_for_redeemed, watch_for_refunded, Deployed,
        Params, Redeemed, Refunded,
    },
    identity, transaction, Secret, SecretHash, Timestamp,
};

#[derive(Debug, Clone, Error)]
#[error("herc20 HTLC was incorrectly funded, expected {expected} but got {got}")]
pub struct IncorrectlyFunded {
    pub expected: asset::Erc20,
    pub got: asset::Erc20,
}

#[async_trait::async_trait]
pub trait WatchForDeployed {
    async fn watch_for_deployed(
        &self,
        params: Params,
        utc_start_of_swap: OffsetDateTime,
    ) -> Deployed;
}

#[async_trait::async_trait]
pub trait WatchForFunded {
    async fn watch_for_funded(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Result<Funded, IncorrectlyFunded>;
}

#[async_trait::async_trait]
pub trait WatchForRedeemed {
    async fn watch_for_redeemed(
        &self,
        params: Params,
        deploy_event: Deployed,
        utc_start_of_swap: OffsetDateTime,
    ) -> Redeemed;
}

#[derive(Debug, Clone)]
pub struct Funded {
    pub transaction: ethereum::Hash,
    pub asset: asset::Erc20,
}

#[cfg(all(test, feature = "testcontainers"))]
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
