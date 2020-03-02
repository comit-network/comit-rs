// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use crate::swap_protocols::rfc003::{create_swap::HtlcParams, ledger::Ledger, Secret};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Funded<T, A> {
    pub transaction: T,
    pub asset: A,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Redeemed<T> {
    pub transaction: T,
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Deployed<T, H> {
    pub transaction: T,
    pub location: H,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Refunded<T> {
    pub transaction: T,
}

#[async_trait::async_trait]
pub trait HtlcFunded<L, A, I>: Send + Sync + Sized + 'static
where
    L: Ledger,
{
    async fn htlc_funded(
        &self,
        htlc_params: &HtlcParams<'_, L, A, I>,
        htlc_deployment: &Deployed<L::Transaction, L::HtlcLocation>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Funded<L::Transaction, A>>;
}

#[async_trait::async_trait]
pub trait HtlcDeployed<L, A, I>: Send + Sync + Sized + 'static
where
    L: Ledger,
{
    async fn htlc_deployed(
        &self,
        htlc_params: &HtlcParams<'_, L, A, I>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Deployed<L::Transaction, L::HtlcLocation>>;
}

#[async_trait::async_trait]
pub trait HtlcRedeemed<L, A, I>: Send + Sync + Sized + 'static
where
    L: Ledger,
{
    async fn htlc_redeemed(
        &self,
        htlc_params: &HtlcParams<'_, L, A, I>,
        htlc_deployment: &Deployed<L::Transaction, L::HtlcLocation>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Redeemed<L::Transaction>>;
}

#[async_trait::async_trait]
pub trait HtlcRefunded<L, A, I>: Send + Sync + Sized + 'static
where
    L: Ledger,
{
    async fn htlc_refunded(
        &self,
        htlc_params: &HtlcParams<'_, L, A, I>,
        htlc_deployment: &Deployed<L::Transaction, L::HtlcLocation>,
        start_of_swap: NaiveDateTime,
    ) -> anyhow::Result<Refunded<L::Transaction>>;
}
