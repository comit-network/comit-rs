pub mod actions;
mod communication_events;
mod spawner;
mod swap_request;

pub use self::{actions::*, communication_events::*, spawner::*, swap_request::SwapRequest};

use crate::{
    comit_client::SwapReject,
    item_cache::ItemCache,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            bob::actions::{Accept, Decline},
            events::{LedgerEvents, ResponseFuture},
            ledger::Ledger,
            role::Initiation,
            save_state::SaveState,
            state_machine::{Context, FutureSwapOutcome, Start, StateMachineResponse, Swap},
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
                    Result<StateMachineResponse<AL::HtlcIdentity, BL::HtlcIdentity>, SwapReject>,
                >,
            >,
        >,
    >,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> Bob<AL, BL, AA, BA> {
    #[allow(clippy::type_complexity)]
    pub fn new_state_machine(
        initiation: Initiation<Self>,
        alpha_ledger_events: Box<dyn LedgerEvents<AL, AA>>,
        beta_ledger_events: Box<dyn LedgerEvents<BL, BA>>,
        save_state: Arc<dyn SaveState<Self>>,
    ) -> (Box<FutureSwapOutcome<Self>>, Box<ResponseFuture<Self>>) {
        let (bob, response_future) = Self::create();

        // We need to duplicate the future
        let (response_for_state_machine, response_for_caller) =
            ItemCache::from_future(response_future).duplicate();

        let start_state = Start {
            alpha_ledger: initiation.alpha_ledger,
            beta_ledger: initiation.beta_ledger,
            alpha_asset: initiation.alpha_asset,
            beta_asset: initiation.beta_asset,
            alpha_ledger_refund_identity: initiation.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: initiation.beta_ledger_redeem_identity,
            alpha_expiry: initiation.alpha_expiry,
            beta_expiry: initiation.beta_expiry,
            secret: initiation.secret,
            role: bob,
        };

        save_state.save(start_state.clone().into());

        let context = Context {
            alpha_ledger_events,
            beta_ledger_events,
            communication_events: Box::new(BobToAlice::new(Box::new(response_for_state_machine))),
            state_repo: save_state,
        };

        (
            Box::new(Swap::start_in(start_state, context)),
            Box::new(response_for_caller),
        )
    }

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
