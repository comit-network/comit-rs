// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use crate::{
    btsieve::Query,
    comit_client::SwapReject,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self, ledger::Ledger, messages::AcceptResponseBody, state_machine::HtlcParams,
            FundTransaction, RedeemTransaction, RefundTransaction,
        },
    },
};
use tokio::{self, prelude::future::Either};

mod btsieve;

pub use self::btsieve::{BtsieveEvents, BtsieveEventsForErc20};

type Future<I> = dyn tokio::prelude::Future<Item = I, Error = rfc003::Error> + Send;

#[allow(type_alias_bounds)]
pub type ResponseFuture<AL, BL> = Future<Result<AcceptResponseBody<AL, BL>, SwapReject>>;

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

pub trait CommunicationEvents<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>: Send {
    fn request_responded(
        &mut self,
        request: &rfc003::messages::Request<AL, BL, AA, BA>,
    ) -> &mut ResponseFuture<AL, BL>;
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
