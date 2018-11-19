// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use ledger_query_service::Query;
use swap_protocols::{
    asset::Asset,
    rfc003::{self, ledger::Ledger, messages::Request},
};
use tokio::{self, prelude::future::Either};

pub use self::lqs::LqsEvents;
mod lqs;
mod response;
use comit_client::SwapReject;
use swap_protocols::rfc003::{
    roles::Role,
    state_machine::{HtlcParams, StateMachineResponse},
};

type Future<I> = tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

pub type StateMachineResponseFuture<SLSI, TLRI, TLLD> =
    Future<Result<StateMachineResponse<SLSI, TLRI, TLLD>, SwapReject>>;

#[allow(type_alias_bounds)]
pub type ResponseFuture<R: Role> = StateMachineResponseFuture<
    R::SourceSuccessHtlcIdentity,
    R::TargetRefundHtlcIdentity,
    <R::TargetLedger as Ledger>::LockDuration,
>;

pub type Funded<L: Ledger> = Future<L::HtlcLocation>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
    Future<Either<SL::Transaction, TL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::Transaction, L::Transaction>>;

pub trait LedgerEvents<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>: Send {
    fn source_htlc_funded(&mut self, htlc_params: HtlcParams<SL, SA>) -> &mut Funded<SL>;

    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        source_htlc_params: HtlcParams<SL, SA>,
        target_htlc_params: HtlcParams<TL, TA>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut SourceRefundedOrTargetFunded<SL, TL>;

    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_params: HtlcParams<SL, SA>,
        htlc_location: &SL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<SL>;

    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_params: HtlcParams<TL, TA>,
        htlc_location: &TL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<TL>;
}

pub trait CommunicationEvents<R: Role> {
    fn request_responded(
        &mut self,
        request: &Request<R::SourceLedger, R::TargetLedger, R::SourceAsset, R::TargetAsset>,
    ) -> &mut ResponseFuture<R>;
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
