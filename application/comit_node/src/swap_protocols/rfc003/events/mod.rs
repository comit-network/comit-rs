// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use crate::{
    comit_client::{self, SwapReject},
    ledger_query_service::Query,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            ledger::Ledger,
            roles::Role,
            state_machine::{HtlcParams, StateMachineResponse},
            FundTransaction, RedeemTransaction, RefundTransaction,
        },
    },
};
use tokio::{self, prelude::future::Either};

mod alice;
mod bob;
mod lqs;

pub use self::{
    alice::AliceToBob,
    bob::BobToAlice,
    lqs::{LqsEvents, LqsEventsForErc20},
};

type Future<I> = tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

pub type StateMachineResponseFuture<ALSI, BLRI, BLLD> =
    Future<Result<StateMachineResponse<ALSI, BLRI, BLLD>, SwapReject>>;

#[allow(type_alias_bounds)]
pub type ResponseFuture<R: Role> = StateMachineResponseFuture<
    R::AlphaRedeemHtlcIdentity,
    R::BetaRefundHtlcIdentity,
    <R::BetaLedger as Ledger>::LockDuration,
>;

pub type Deployed<L: Ledger> = Future<L::HtlcLocation>;
pub type Funded<L: Ledger> = Future<Option<FundTransaction<L>>>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type AlphaRefundedOrBetaFunded<AL: Ledger, BL: Ledger> =
    Future<Either<AL::Transaction, BL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<RedeemTransaction<L>, RefundTransaction<L>>>;

pub trait LedgerEvents<L: Ledger, A: Asset>: Send {
    fn htlc_deployed(&mut self, htlc_params: HtlcParams<L, A>) -> &mut Deployed<L>;

    fn htlc_funded(
        &mut self,
        htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut Funded<L>;

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

pub trait NewHtlcDeployedQuery<L: Ledger, A: Asset>: Send + Sync
where
    Self: Query,
{
    fn new_htlc_deployed_query(htlc_params: &HtlcParams<L, A>) -> Self;
}

pub trait NewHtlcFundedQuery<L: Ledger, A: Asset>: Send + Sync
where
    Self: Query,
{
    fn new_htlc_funded_query(htlc_params: &HtlcParams<L, A>) -> Self;
}

pub trait NewHtlcRedeemedQuery<L: Ledger, A: Asset>: Send + Sync
where
    Self: Query,
{
    fn new_htlc_redeemed_query(
        htlc_params: &HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> Self;
}

pub trait NewHtlcRefundedQuery<L: Ledger, A: Asset>: Send + Sync
where
    Self: Query,
{
    fn new_htlc_refunded_query(
        htlc_params: &HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> Self;
}
