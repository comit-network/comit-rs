pub mod bitcoin;
pub mod erc20;
pub mod ether;

use crate::{
    comit_client::{SwapDeclineReason, SwapReject},
    swap_protocols::{
        asset::Asset,
        rfc003::{
            bob::ResponseSender, messages::IntoAcceptResponseBody, secret_source::SecretSource,
            state_machine::HtlcParams, Ledger, Secret,
        },
    },
};
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
    sender: ResponseSender<AL, BL>,
    #[derivative(Debug = "ignore")]
    secret_source: Arc<dyn SecretSource>,
}

impl<AL: Ledger, BL: Ledger> Accept<AL, BL> {
    #[allow(clippy::type_complexity)]
    pub fn new(sender: ResponseSender<AL, BL>, secret_source: Arc<dyn SecretSource>) -> Self {
        Self {
            sender,
            secret_source,
        }
    }
    pub fn accept<P: IntoAcceptResponseBody<AL, BL>>(&self, partial_response: P) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();

        match sender.take() {
            Some(sender) => {
                sender
                    .send(Ok(
                        partial_response.into_accept_response_body(self.secret_source.as_ref())
                    ))
                    .expect("Action shouldn't outlive BobToAlice");
                Ok(())
            }
            None => Err(()),
        }
    }
}

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct Decline<AL: Ledger, BL: Ledger> {
    #[derivative(Debug = "ignore")]
    sender: ResponseSender<AL, BL>,
}

impl<AL: Ledger, BL: Ledger> Decline<AL, BL> {
    pub fn new(sender: ResponseSender<AL, BL>) -> Self {
        Self { sender }
    }

    pub fn decline(&self, reason: Option<SwapDeclineReason>) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();
        match sender.take() {
            Some(sender) => {
                sender
                    .send(Err(SwapReject::Declined { reason }))
                    .expect("Action shouldn't outlive BobToAlice");
                Ok(())
            }
            None => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn action_kind_serializes_into_lowercase_str() {
        assert_eq!(format!("{}", ActionKind::Accept), "accept".to_string());
        assert_eq!(format!("{}", ActionKind::Decline), "decline".to_string());
        assert_eq!(format!("{}", ActionKind::Fund), "fund".to_string());
        assert_eq!(format!("{}", ActionKind::Refund), "refund".to_string());
        assert_eq!(format!("{}", ActionKind::Redeem), "redeem".to_string());
        assert_eq!(format!("{}", ActionKind::Deploy), "deploy".to_string());
    }
}
