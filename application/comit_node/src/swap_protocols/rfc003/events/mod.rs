// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use comit_client::SwapReject;
use swap_protocols::rfc003::{
    self,
    ledger::Ledger,
    messages::{AcceptResponse, Request},
    SecretHash,
};
use tokio::{self, prelude::future::Either};

mod default;

pub use self::default::{DefaultEvents, Player};
use ledger_query_service::Query;
use swap_protocols::{asset::Asset, rfc003::state_machine::OngoingSwap};

type Future<I> = tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

pub type Response<SL, TL> = Future<Result<AcceptResponse<SL, TL>, SwapReject>>;
pub type Funded<L: Ledger> = Future<L::HtlcLocation>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
    Future<Either<SL::TxId, TL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::TxId, L::TxId>>;

pub trait RequestResponded<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>: Send {
    fn request_responded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<Response<SL, TL>>;
}

pub trait SourceHtlcFunded<
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Clone,
>: Send
{
    fn source_htlc_funded(&mut self, swap: &OngoingSwap<SL, TL, SA, TA, S>)
        -> &mut Box<Funded<SL>>;
}

pub trait SourceHtlcRefundedTargetHtlcFunded<
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Clone,
>: Send
{
    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut Box<SourceRefundedOrTargetFunded<SL, TL>>;
}

pub trait TargetHtlcRedeemedOrRefunded<TL: Ledger>: Send {
    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_location: &TL::HtlcLocation,
    ) -> &mut Box<RedeemedOrRefunded<TL>>;
}

pub trait SourceHtlcRedeemedOrRefunded<SL: Ledger>: Send {
    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut Box<RedeemedOrRefunded<SL>>;
}

pub trait Events<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset, S: Into<SecretHash> + Clone>:
    RequestResponded<SL, TL, SA, TA>
    + SourceHtlcFunded<SL, TL, SA, TA, S>
    + SourceHtlcRefundedTargetHtlcFunded<SL, TL, SA, TA, S>
    + TargetHtlcRedeemedOrRefunded<TL>
    + SourceHtlcRedeemedOrRefunded<SL>
{
}

pub trait NewSourceHtlcFundedQuery<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Clone,
    Self: Query,
{
    fn new_source_htlc_funded_query(swap: &OngoingSwap<SL, TL, SA, TA, S>) -> Self;
}

pub trait NewSourceHtlcRedeemedQuery<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Clone,
    Self: Query,
{
    fn new_source_htlc_redeemed_query(swap: &OngoingSwap<SL, TL, SA, TA, S>) -> Self;
}
pub trait NewSourceHtlcRefundedQuery<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Clone,
    Self: Query,
{
    fn new_source_htlc_refunded_query(
        swap: &OngoingSwap<SL, TL, SA, TA, S>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> Self;
}

pub trait NewTargetHtlcFundedQuery<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Clone,
    Self: Query,
{
    fn new_target_htlc_funded_query(swap: &OngoingSwap<SL, TL, SA, TA, S>) -> Self;
}

pub trait NewTargetHtlcRedeemedQuery<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Clone,
    Self: Query,
{
    fn new_target_htlc_redeemed_query(swap: &OngoingSwap<SL, TL, SA, TA, S>) -> Self;
}
pub trait NewTargetHtlcRefundedQuery<SL, TL, SA, TA, S>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Clone,
    Self: Query,
{
    fn new_target_htlc_refunded_query(swap: &OngoingSwap<SL, TL, SA, TA, S>) -> Self;
}
