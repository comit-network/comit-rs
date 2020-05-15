use crate::{
    asset, htlc_location,
    swap_protocols::{hbit::HtlcParams, Secret},
    transaction,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum Funded {
    Correctly {
        asset: asset::Bitcoin,
        transaction: transaction::Bitcoin,
    },
    Incorrectly {
        asset: asset::Bitcoin,
        transaction: transaction::Bitcoin,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Redeemed {
    pub transaction: transaction::Bitcoin,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployed {
    pub location: htlc_location::Bitcoin,
    pub transaction: transaction::Bitcoin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refunded {
    pub transaction: transaction::Bitcoin,
}

#[async_trait::async_trait]
pub trait HtlcFunded: Send + Sync + Sized + 'static {
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams,
        htlc_deployment: &Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded>;
}

#[async_trait::async_trait]
pub trait HtlcDeployed {
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed>;
}

#[async_trait::async_trait]
pub trait HtlcRedeemed: Send + Sync + Sized + 'static {
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams,
        htlc_deployment: &Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed>;
}

#[async_trait::async_trait]
pub trait HtlcRefunded: Send + Sync + Sized + 'static {
    async fn htlc_refunded(
        &self,
        htlc_params: &HtlcParams,
        htlc_deployment: &Deployed,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded>;
}
