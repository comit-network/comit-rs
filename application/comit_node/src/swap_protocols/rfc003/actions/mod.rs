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

#[derive(Debug, strum_macros::EnumDiscriminants)]
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

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct Accept<AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    sender: ResponseSender<AL, BL>,
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

#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct Decline<AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    sender: ResponseSender<AL, BL>,
}

impl<AL: Ledger, BL: Ledger> Decline<AL, BL> {
    #[allow(clippy::type_complexity)]
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
