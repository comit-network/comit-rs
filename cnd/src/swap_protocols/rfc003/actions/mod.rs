pub mod bitcoin;
pub mod erc20;
pub mod ether;

use crate::swap_protocols::{
    asset::Asset,
    rfc003::{
        messages::{
            AcceptResponseBody, DeclineResponseBody, IntoAcceptResponseBody, SwapDeclineReason,
        },
        secret_source::SecretSource,
        state_machine::HtlcParams,
        Ledger, Secret,
    },
};
use failure::_core::marker::PhantomData;
use std::sync::Arc;

/// Defines the set of actions available in the RFC003 protocol
#[derive(Debug, Clone, PartialEq, strum_macros::EnumDiscriminants)]
#[strum_discriminants(
    name(ActionKind),
    derive(Display, EnumString),
    strum(serialize_all = "snake_case")
)]
pub enum Action<Accept, Decline, Deploy, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

pub trait FundAction<L: Ledger, A: Asset> {
    type FundActionOutput;

    fn fund_action(htlc_params: HtlcParams<L, A>) -> Self::FundActionOutput;
}

pub trait RefundAction<L: Ledger, A: Asset> {
    type RefundActionOutput;

    fn refund_action(
        htlc_params: HtlcParams<L, A>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn SecretSource,
        fund_transaction: &L::Transaction,
    ) -> Self::RefundActionOutput;
}

pub trait RedeemAction<L: Ledger, A: Asset> {
    type RedeemActionOutput;

    fn redeem_action(
        htlc_params: HtlcParams<L, A>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput;
}

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct Accept<AL: Ledger, BL: Ledger> {
    #[derivative(Debug = "ignore")]
    secret_source: Arc<dyn SecretSource>,
    phantom_data: PhantomData<(AL, BL)>,
}

impl<AL: Ledger, BL: Ledger> Accept<AL, BL> {
    #[allow(clippy::type_complexity)]
    pub fn new(secret_source: Arc<dyn SecretSource>) -> Self {
        Self {
            secret_source,
            phantom_data: PhantomData,
        }
    }

    pub fn accept<P: IntoAcceptResponseBody<AL, BL>>(
        &self,
        partial_response: P,
    ) -> AcceptResponseBody<AL, BL> {
        partial_response.into_accept_response_body(self.secret_source.as_ref())
    }
}

#[derive(Clone, derivative::Derivative, Default)]
#[derivative(Debug)]
pub struct Decline<AL: Ledger, BL: Ledger> {
    phantom_data: PhantomData<(AL, BL)>,
}

impl<AL: Ledger, BL: Ledger> Decline<AL, BL> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }

    pub fn decline(&self, reason: Option<SwapDeclineReason>) -> DeclineResponseBody {
        DeclineResponseBody { reason }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn action_kind_serializes_into_lowercase_str() {
        assert_eq!(ActionKind::Accept.to_string(), "accept".to_string());
        assert_eq!(ActionKind::Decline.to_string(), "decline".to_string());
        assert_eq!(ActionKind::Fund.to_string(), "fund".to_string());
        assert_eq!(ActionKind::Refund.to_string(), "refund".to_string());
        assert_eq!(ActionKind::Redeem.to_string(), "redeem".to_string());
        assert_eq!(ActionKind::Deploy.to_string(), "deploy".to_string());
    }
}
