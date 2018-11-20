// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use comit_client;
use ledger_query_service::Query;
use swap_protocols::{
    asset::Asset,
    rfc003::{self, ledger::Ledger},
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

pub type StateMachineResponseFuture<ALSI, BLRI, BLLD> =
    Future<Result<StateMachineResponse<ALSI, BLRI, BLLD>, SwapReject>>;

#[allow(type_alias_bounds)]
pub type ResponseFuture<R: Role> = StateMachineResponseFuture<
    R::AlphaSuccessHtlcIdentity,
    R::BetaRefundHtlcIdentity,
    <R::BetaLedger as Ledger>::LockDuration,
>;

pub type Funded<L: Ledger> = Future<L::HtlcLocation>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type AlphaRefundedOrBetaFunded<AL: Ledger, BL: Ledger> =
    Future<Either<AL::Transaction, BL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::Transaction, L::Transaction>>;

pub trait LedgerEvents<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>: Send {
    fn alpha_htlc_funded(&mut self, htlc_params: HtlcParams<AL, AA>) -> &mut Funded<AL>;

    fn alpha_htlc_refunded_beta_htlc_funded(
        &mut self,
        alpha_htlc_params: HtlcParams<AL, AA>,
        beta_htlc_params: HtlcParams<BL, BA>,
        alpha_htlc_location: &AL::HtlcLocation,
    ) -> &mut AlphaRefundedOrBetaFunded<AL, BL>;

    fn alpha_htlc_redeemed_or_refunded(
        &mut self,
        alpha_htlc_params: HtlcParams<AL, AA>,
        htlc_location: &AL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<AL>;

    fn beta_htlc_redeemed_or_refunded(
        &mut self,
        beta_htlc_params: HtlcParams<BL, BA>,
        htlc_location: &BL::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<BL>;
}

pub trait CommunicationEvents<R: Role> {
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
