// This is fine because we're using associated types
// see: https://github.com/rust-lang/rust/issues/21903
#![allow(type_alias_bounds)]

use comit_client::SwapReject;
use swap_protocols::rfc003::{self, ledger::Ledger, messages::AcceptResponse};
use tokio::{self, prelude::future::Either};

type Future<I> = tokio::prelude::future::Future<Item = I, Error = rfc003::Error> + Send;

pub type Response<SL, TL> = Future<Result<AcceptResponse<SL, TL>, SwapReject>>;
pub type Funded<L: Ledger> = Future<L::HtlcLocation>;
pub type Refunded<L: Ledger> = Future<L::TxId>;
pub type Redeemed<L: Ledger> = Future<L::TxId>;
pub type SourceRefundedOrTargetFunded<SL: Ledger, TL: Ledger> =
    Future<Either<SL::TxId, TL::HtlcLocation>>;
pub type RedeemedOrRefunded<L: Ledger> = Future<Either<L::TxId, L::TxId>>;
