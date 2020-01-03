// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use crate::swap_protocols::{
    asset::Asset,
    rfc003::{self, create_swap::HtlcParams, ledger::Ledger, Secret},
};
use futures_core::future::Either;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Funded<L: Ledger, A: Asset> {
    pub transaction: L::Transaction,
    pub asset: A,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Redeemed<L: Ledger> {
    pub transaction: L::Transaction,
    pub secret: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Deployed<L: Ledger> {
    pub transaction: L::Transaction,
    pub location: L::HtlcLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Refunded<L: Ledger> {
    pub transaction: L::Transaction,
}

#[async_trait::async_trait]
pub trait HtlcEvents<L: Ledger, A: Asset>: Send + Sync + 'static {
    async fn htlc_deployed(
        &self,
        htlc_params: HtlcParams<L, A>,
    ) -> Result<Deployed<L>, rfc003::Error>;
    async fn htlc_funded(
        &self,
        htlc_params: HtlcParams<L, A>,
        htlc_deployment: &Deployed<L>,
    ) -> Result<Funded<L, A>, rfc003::Error>;
    async fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<L, A>,
        htlc_deployment: &Deployed<L>,
        htlc_funding: &Funded<L, A>,
    ) -> Result<Either<Redeemed<L>, Refunded<L>>, rfc003::Error>;
}
