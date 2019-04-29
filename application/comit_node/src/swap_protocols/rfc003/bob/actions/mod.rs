mod erc20;
mod generic_impl;

use crate::{
    comit_client::{SwapDeclineReason, SwapReject},
    swap_protocols::rfc003::{
        actions::Action, bob::ResponseSender, messages::ToAcceptResponseBody,
        secret_source::SecretSource, Ledger,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund>
    ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
{
    fn into_action(self) -> Action<ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>> {
        Action {
            inner: self,
            invalid_until: None,
        }
    }
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
    pub fn accept<P: ToAcceptResponseBody<AL, BL>>(&self, partial_response: P) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();

        match sender.take() {
            Some(sender) => {
                sender
                    .send(Ok(
                        partial_response.to_accept_response_body(self.secret_source.as_ref())
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
