mod btc_erc20;
mod btc_eth;
mod eth_btc;

use crate::{
    comit_client::{SwapDeclineReason, SwapReject},
    swap_protocols::rfc003::{state_machine::StateMachineResponse, Ledger},
};
use futures::sync::oneshot;
use std::sync::{Arc, Mutex};

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
    pub fn name(&self) -> String {
        use self::ActionKind::*;
        match *self {
            Accept(_) => String::from("accept"),
            Decline(_) => String::from("decline"),
            Deploy(_) => String::from("deploy"),
            Fund(_) => String::from("fund"),
            Redeem(_) => String::from("redeem"),
            Refund(_) => String::from("refund"),
        }
    }
}

#[allow(type_alias_bounds)]
type Response<AL: Ledger, BL: Ledger> =
    Result<StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>, SwapReject>;

#[derive(Debug, Clone)]
pub struct Accept<AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>,
}

impl<AL: Ledger, BL: Ledger> Accept<AL, BL> {
    #[allow(clippy::type_complexity)]
    pub fn new(sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>) -> Self {
        Self { sender }
    }
    pub fn accept(
        &self,
        response: StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>,
    ) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();

        match sender.take() {
            Some(sender) => {
                sender
                    .send(Ok(response))
                    .expect("Action shouldn't outlive BobToAlice");
                Ok(())
            }
            None => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Decline<AL: Ledger, BL: Ledger> {
    #[allow(clippy::type_complexity)]
    sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>,
}

impl<AL: Ledger, BL: Ledger> Decline<AL, BL> {
    #[allow(clippy::type_complexity)]
    pub fn new(sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>) -> Self {
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
