pub mod actions;
mod communication_events;
mod handler;
mod swap_request;
mod swap_response;

pub use self::{
    communication_events::*,
    handler::SwapRequestHandler,
    swap_request::{SwapRequest, SwapRequestKind},
    swap_response::SwapResponseKind,
};

use crate::{
    comit_client::SwapReject,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            bob::actions::{Accept, Decline},
            events::ResponseFuture,
            ledger::Ledger,
            state_machine::StateMachineResponse,
            Role, SecretHash,
        },
    },
};

use futures::{sync::oneshot, Future};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct Bob<AL: Ledger, BL: Ledger, AA, BA> {
    phantom_data: PhantomData<(AL, BL, AA, BA)>,
    #[allow(clippy::type_complexity)]
    response_sender: Arc<
        Mutex<
            Option<
                oneshot::Sender<
                    Result<
                        StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity, BL::LockDuration>,
                        SwapReject,
                    >,
                >,
            >,
        >,
    >,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Bob<AL, BL, AA, BA> {
    pub fn create() -> (Self, Box<ResponseFuture<Self>>) {
        let (sender, receiver) = oneshot::channel();
        (
            Bob {
                phantom_data: PhantomData,
                response_sender: Arc::new(Mutex::new(Some(sender))),
            },
            Box::new(
                receiver
                    .map_err(|_e| unreachable!("For now, it should be impossible for the sender to go out of scope before the receiver") ),
            ),
        )
    }

    pub fn accept_action(&self) -> Accept<AL, BL> {
        Accept::new(self.response_sender.clone())
    }

    pub fn decline_action(&self) -> Decline<AL, BL> {
        Decline::new(self.response_sender.clone())
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Role for Bob<AL, BL, AA, BA> {
    type AlphaLedger = AL;
    type BetaLedger = BL;
    type AlphaAsset = AA;
    type BetaAsset = BA;
    type AlphaRedeemHtlcIdentity = AL::HtlcIdentity;
    type AlphaRefundHtlcIdentity = AL::Identity;
    type BetaRedeemHtlcIdentity = BL::Identity;
    type BetaRefundHtlcIdentity = BL::HtlcIdentity;
    type Secret = SecretHash;
}
