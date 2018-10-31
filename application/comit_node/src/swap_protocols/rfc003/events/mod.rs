// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use comit_client::SwapReject;
use swap_protocols::rfc003::{
    self,
    ledger::Ledger,
    messages::{AcceptResponse, Request},
    state_machine::Start,
    SecretHash,
};
use tokio::{self, prelude::future::Either};

mod default;

pub use self::default::{DefaultEvents, Player};
use ledger_query_service::Query;
use swap_protocols::asset::Asset;

type Future<I> = tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

pub type Response<SL, TL> = Future<Result<AcceptResponse<SL, TL>, SwapReject>>;
pub type Funded<L: Ledger> = Future<L::HtlcLocation>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
    Future<Either<SL::TxId, TL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::TxId, L::TxId>>;

pub trait RequestResponded<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone>: Send {
    fn request_responded(
        &mut self,
        request: &Request<SL, TL, SA, TA>,
    ) -> &mut Box<Response<SL, TL>>;
}

pub trait SourceHtlcFunded<
    SL: Ledger,
    TL: Ledger,
    SA: Clone,
    TA: Clone,
    S: Into<SecretHash> + Clone,
>: Send
{
    fn source_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA, S>,
        response: &AcceptResponse<SL, TL>,
    ) -> &mut Box<Funded<SL>>;
}

pub trait SourceHtlcRefundedTargetHtlcFunded<
    SL: Ledger,
    TL: Ledger,
    SA: Clone,
    TA: Clone,
    S: Into<SecretHash> + Clone,
>: Send
{
    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        start: &Start<SL, TL, SA, TA, S>,
        response: &AcceptResponse<SL, TL>,
        source_htlc_id: &SL::HtlcLocation,
    ) -> &mut Box<SourceRefundedOrTargetFunded<SL, TL>>;
}

pub trait TargetHtlcRedeemedOrRefunded<TL: Ledger>: Send {
    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_id: &TL::HtlcLocation,
    ) -> &mut Box<RedeemedOrRefunded<TL>>;
}

pub trait SourceHtlcRedeemedOrRefunded<SL: Ledger>: Send {
    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_id: &SL::HtlcLocation,
    ) -> &mut Box<RedeemedOrRefunded<SL>>;
}

pub trait Events<SL: Ledger, TL: Ledger, SA: Clone, TA: Clone, S: Into<SecretHash> + Clone>:
    RequestResponded<SL, TL, SA, TA>
    + SourceHtlcFunded<SL, TL, SA, TA, S>
    + SourceHtlcRefundedTargetHtlcFunded<SL, TL, SA, TA, S>
    + TargetHtlcRedeemedOrRefunded<TL>
    + SourceHtlcRedeemedOrRefunded<SL>
{
}

pub trait QueryFactory<SL, TL, SA, TA, S, Q>: Send + Sync
where
    SL: Ledger,
    TL: Ledger,
    SA: Asset,
    TA: Asset,
    S: Into<SecretHash> + Send + Sync + Clone,
{
    fn create(&self, start: &Start<SL, TL, SA, TA, S>, response: &AcceptResponse<SL, TL>) -> Q;
}
