mod btc_erc20;
mod btc_eth;
use comit_client::SwapReject;
use futures::sync::oneshot;
use std::sync::{Arc, Mutex};
use swap_protocols::rfc003::{state_machine::StateMachineResponse, Ledger};

#[allow(type_alias_bounds)]
type Response<AL: Ledger, BL: Ledger> =
    Result<StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>, SwapReject>;

#[derive(Debug, Clone)]
pub struct Accept<AL: Ledger, BL: Ledger> {
    sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>,
}

impl<AL: Ledger, BL: Ledger> Accept<AL, BL> {
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
    sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>,
}

impl<AL: Ledger, BL: Ledger> Decline<AL, BL> {
    pub fn new(sender: Arc<Mutex<Option<oneshot::Sender<Response<AL, BL>>>>>) -> Self {
        Self { sender }
    }
    pub fn decline(&self) -> Result<(), ()> {
        let mut sender = self.sender.lock().unwrap();
        match sender.take() {
            Some(sender) => {
                sender // TODO: Implement SwapReject::Decline(reason)
                    .send(Err(SwapReject::Rejected))
                    .expect("Action shouldn't outlive BobToAlice");
                Ok(())
            }
            None => Err(()),
        }
    }
}
