pub use crate::herc20::{Deployed, Params, Redeemed};

use crate::{asset, ethereum};
use anyhow::Result;
use thiserror::Error;
use time::OffsetDateTime;

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
