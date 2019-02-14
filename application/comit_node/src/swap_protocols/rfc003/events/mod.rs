// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use crate::{
    comit_client::{self, SwapReject},
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            ledger::Ledger,
            state_machine::{HtlcParams, StateMachineResponse},
            Role, Secret,
        },
    },
};
use tokio::{self, prelude::future::Either};

mod ledger_event_futures;

pub use self::ledger_event_futures::*;

type Future<I> = dyn tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

pub type StateMachineResponseFuture<ALSI, BLRI> =
    Future<Result<StateMachineResponse<ALSI, BLRI>, SwapReject>>;

#[allow(type_alias_bounds)]
pub type ResponseFuture<R: Role> =
    StateMachineResponseFuture<R::AlphaRedeemHtlcIdentity, R::BetaRefundHtlcIdentity>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundTransaction<L: Ledger, A: Asset> {
    pub transaction: L::Transaction,
    pub asset: A,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedeemTransaction<L: Ledger> {
    pub transaction: L::Transaction,
    pub secret: Secret,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployTransaction<L: Ledger> {
    pub transaction: L::Transaction,
    pub location: L::HtlcLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefundTransaction<L: Ledger>(pub L::Transaction);

pub type Deployed<L: Ledger> = Future<DeployTransaction<L>>;
pub type Funded<L: Ledger, A: Asset> = Future<FundTransaction<L, A>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<RedeemTransaction<L>, RefundTransaction<L>>>;

pub trait LedgerEvents<L: Ledger, A: Asset>: Send {
    fn htlc_deployed(&mut self, htlc_params: HtlcParams<L, A>) -> &mut Deployed<L>;

    fn htlc_funded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_deployment: &DeployTransaction<L>,
    ) -> &mut Funded<L, A>;

    fn htlc_redeemed_or_refunded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<L>;
}

pub trait CommunicationEvents<R: Role>: Send {
    fn request_responded(
        &mut self,
        request: &comit_client::rfc003::Request<
            R::AlphaLedger,
            R::BetaLedger,
            R::AlphaAsset,
            R::BetaAsset,
        >,
    ) -> &mut ResponseFuture<R>;
}

pub trait HtlcEvents<L: Ledger, A: Asset>: Send + Sync + 'static {
    fn htlc_deployed(&self, htlc_params: HtlcParams<L, A>) -> Box<Deployed<L>>;
    fn htlc_funded(
        &self,
        htlc_params: HtlcParams<L, A>,
        htlc_deployment: &DeployTransaction<L>,
    ) -> Box<Funded<L, A>>;
    fn htlc_redeemed_or_refunded(
        &self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> Box<RedeemedOrRefunded<L>>;
}
