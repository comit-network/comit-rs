pub mod actions;
mod communication_events;
mod spawner;

pub use self::{actions::*, communication_events::*, spawner::*};

use crate::{
    comit_client::SwapReject,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            bob::actions::{Accept, Decline},
            events::{LedgerEvents, ResponseFuture},
            ledger::Ledger,
            ledger_state::LedgerState,
            messages::{AcceptResponseBody, Request},
            save_state::SaveState,
            secret_source::SecretSource,
            state_machine::{Context, FutureSwapOutcome, Start, Swap},
            ActorState, Secret,
        },
    },
};
use futures::{future::Shared, sync::oneshot, Future};
use std::sync::{Arc, Mutex};

#[allow(type_alias_bounds)]
pub type ResponseSender<AL: Ledger, BL: Ledger> =
    Arc<Mutex<Option<oneshot::Sender<Result<AcceptResponseBody<AL, BL>, SwapReject>>>>>;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct State<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub swap_communication: SwapCommunication<AL, BL, AA, BA>,
    pub alpha_ledger_state: LedgerState<AL>,
    pub beta_ledger_state: LedgerState<BL>,
    #[derivative(Debug = "ignore")]
    pub secret_source: Arc<dyn SecretSource>,
    pub secret: Option<Secret>,
    pub error: Option<rfc003::Error>,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub enum SwapCommunication<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    Proposed {
        request: Request<AL, BL, AA, BA>,
        #[derivative(Debug = "ignore")]
        pending_response: PendingResponse<AL, BL>,
    },
    Accepted {
        request: Request<AL, BL, AA, BA>,
        response: AcceptResponseBody<AL, BL>,
    },
    Rejected {
        request: Request<AL, BL, AA, BA>,
        response: SwapReject,
    },
}

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct PendingResponse<AL: Ledger, BL: Ledger> {
    sender: ResponseSender<AL, BL>,
    receiver: Shared<Box<ResponseFuture<AL, BL>>>,
}

impl<AL: Ledger, BL: Ledger> PendingResponse<AL, BL> {
    fn response_future(&self) -> Box<ResponseFuture<AL, BL>> {
        Box::new(self.receiver.clone().then(|result| match result {
            Ok(response) => Ok((*response).clone()),
            Err(e) => Err((*e).clone()),
        }))
    }

    pub fn accept_action(&self, secret_source: Arc<dyn SecretSource>) -> Accept<AL, BL> {
        Accept::new(self.sender.clone(), secret_source)
    }

    pub fn decline_action(&self) -> Decline<AL, BL> {
        Decline::new(self.sender.clone())
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> State<AL, BL, AA, BA> {
    #[allow(clippy::type_complexity)]
    pub fn new_state_machine(
        &self,
        alpha_ledger_events: Box<dyn LedgerEvents<AL, AA>>,
        beta_ledger_events: Box<dyn LedgerEvents<BL, BA>>,
        save_state: Arc<dyn SaveState<AL, BL, AA, BA>>,
    ) -> Box<FutureSwapOutcome<AL, BL, AA, BA>> {
        let response_future = self
            .response_future()
            .expect("A new state machine is only created when a swap has been proposed");
        let communication_events = Box::new(BobToAlice::new(Box::new(response_future)));

        let swap_request = self.request();
        let start_state = Start {
            alpha_ledger: swap_request.alpha_ledger,
            beta_ledger: swap_request.beta_ledger,
            alpha_asset: swap_request.alpha_asset,
            beta_asset: swap_request.beta_asset,
            alpha_ledger_refund_identity: swap_request.alpha_ledger_refund_identity,
            beta_ledger_redeem_identity: swap_request.beta_ledger_redeem_identity,
            alpha_expiry: swap_request.alpha_expiry,
            beta_expiry: swap_request.beta_expiry,
            secret_hash: swap_request.secret_hash,
        };

        let context = Context {
            alpha_ledger_events,
            beta_ledger_events,
            communication_events,
            state_repo: save_state,
        };

        Box::new(Swap::start_in(start_state, context))
    }

    pub fn new(request: Request<AL, BL, AA, BA>, secret_source: Arc<dyn SecretSource>) -> Self {
        let (sender, receiver) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some(sender)));
        let receiver = receiver.map_err(|_e| {
            rfc003::Error::Internal(String::from(
                "For now, it should be impossible for the sender to go out of scope before the receiver",
            ))
        });
        let receiver = (Box::new(receiver) as Box<ResponseFuture<AL, BL>>).shared();

        State {
            swap_communication: SwapCommunication::Proposed {
                request,
                pending_response: PendingResponse { sender, receiver },
            },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source,
            secret: None,
            error: None,
        }
    }

    pub fn request(&self) -> Request<AL, BL, AA, BA> {
        match &self.swap_communication {
            SwapCommunication::Accepted { request, .. }
            | SwapCommunication::Proposed { request, .. }
            | SwapCommunication::Rejected { request, .. } => request.clone(),
        }
    }

    pub fn response_future(&self) -> Option<Box<ResponseFuture<AL, BL>>> {
        match &self.swap_communication {
            SwapCommunication::Proposed {
                pending_response, ..
            } => Some(Box::new(pending_response.response_future())),
            _ => {
                warn!("Swap not in proposed state: {:?}", self);
                None
            }
        }
    }
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> ActorState for State<AL, BL, AA, BA> {
    type AL = AL;
    type BL = BL;
    type AA = AA;
    type BA = BA;

    fn set_response(&mut self, response: Result<AcceptResponseBody<AL, BL>, SwapReject>) {
        match self.swap_communication {
            SwapCommunication::Proposed { ref request, .. } => match response {
                Ok(response) => {
                    self.swap_communication = SwapCommunication::Accepted {
                        request: request.clone(),
                        response,
                    }
                }
                Err(response) => {
                    self.swap_communication = SwapCommunication::Rejected {
                        request: request.clone(),
                        response,
                    }
                }
            },
            _ => error!("Tried to set a response after it's already set"),
        }
    }

    fn set_secret(&mut self, secret: Secret) {
        self.secret = Some(secret)
    }

    fn set_error(&mut self, error: rfc003::Error) {
        self.error = Some(error)
    }

    fn alpha_ledger_mut(&mut self) -> &mut LedgerState<AL> {
        &mut self.alpha_ledger_state
    }

    fn beta_ledger_mut(&mut self) -> &mut LedgerState<BL> {
        &mut self.beta_ledger_state
    }
}
