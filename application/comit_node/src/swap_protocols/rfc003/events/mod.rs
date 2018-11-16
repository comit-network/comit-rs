// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use ledger_query_service::Query;
use swap_protocols::{
    asset::Asset,
    rfc003::{self, ledger::Ledger},
};
use tokio::{self, prelude::future::Either};

pub use self::default::DefaultEvents;
mod default;
use swap_protocols::rfc003::state_machine::HtlcParams;

type Future<I> = tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

pub type Funded<L: Ledger> = Future<L::HtlcLocation>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
    Future<Either<SL::Transaction, TL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::Transaction, L::Transaction>>;

pub trait HtlcFunded<L: Ledger, A: Asset>: Send {
    fn htlc_funded(&mut self, htlc_params: HtlcParams<L, A>) -> &mut Funded<L>;
}

pub trait SourceHtlcRefundedTargetHtlcFunded<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>:
    Send
{
    fn source_htlc_refunded_target_htlc_funded(
        &mut self,
        source_htlc_params: HtlcParams<SL, SA>,
        target_htlc_params: HtlcParams<TL, TA>,
        source_htlc_location: &SL::HtlcLocation,
    ) -> &mut SourceRefundedOrTargetFunded<SL, TL>;
}

pub trait SourceHtlcRedeemedOrRefunded<L: Ledger, A: Asset>: Send {
    fn source_htlc_redeemed_or_refunded(
        &mut self,
        source_htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<L>;
}

pub trait TargetHtlcRedeemedOrRefunded<L: Ledger, A: Asset>: Send {
    fn target_htlc_redeemed_or_refunded(
        &mut self,
        target_htlc_params: HtlcParams<L, A>,
        htlc_location: &L::HtlcLocation,
    ) -> &mut RedeemedOrRefunded<L>;
}

pub trait Events<SL: Ledger, TL: Ledger, SA: Asset, TA: Asset>:
    HtlcFunded<SL, SA>
    + SourceHtlcRefundedTargetHtlcFunded<SL, TL, SA, TA>
    + SourceHtlcRedeemedOrRefunded<SL, SA>
    + TargetHtlcRedeemedOrRefunded<TL, TA>
{
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
        source_htlc_location: &L::HtlcLocation,
    ) -> Self;
}

pub trait NewHtlcRefundedQuery<L: Ledger, A: Asset>: Send + Sync
where
    Self: Query,
{
    fn new_htlc_refunded_query(
        htlc_params: &HtlcParams<L, A>,
        source_htlc_location: &L::HtlcLocation,
    ) -> Self;
}
