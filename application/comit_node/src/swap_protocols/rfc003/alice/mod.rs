mod actions;
mod communication_events;
mod spawner;

pub use self::{actions::*, communication_events::*, spawner::*};

use crate::{
    comit_client::{self, SwapReject},
    swap_protocols::{
        asset::Asset,
        rfc003::{
            self,
            events::LedgerEvents,
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
use derivative::Derivative;
use libp2p::PeerId;
use std::sync::Arc;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq)]
pub struct State<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub swap_communication: SwapCommunication<AL, BL, AA, BA>,
    pub alpha_ledger_state: LedgerState<AL>,
    pub beta_ledger_state: LedgerState<BL>,
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub secret_source: Arc<dyn SecretSource>,
    pub error: Option<rfc003::Error>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwapCommunication<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    Proposed {
        request: Request<AL, BL, AA, BA>,
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

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> State<AL, BL, AA, BA> {
    pub fn new_state_machine<C: comit_client::Client>(
        &self,
        alpha_ledger_events: Box<dyn LedgerEvents<AL, AA>>,
        beta_ledger_events: Box<dyn LedgerEvents<BL, BA>>,
        comit_client: Arc<C>,
        bob_id: PeerId,
        save_state: Arc<dyn SaveState<AL, BL, AA, BA>>,
    ) -> Box<FutureSwapOutcome<AL, BL, AA, BA>> {
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
            communication_events: Box::new(AliceToBob::new(comit_client, bob_id)),
            state_repo: save_state,
        };

        Box::new(Swap::start_in(start_state, context))
    }

    pub fn request(&self) -> Request<AL, BL, AA, BA> {
        match &self.swap_communication {
            SwapCommunication::Accepted { request, .. }
            | SwapCommunication::Proposed { request }
            | SwapCommunication::Rejected { request, .. } => request.clone(),
        }
    }

    pub fn new(request: Request<AL, BL, AA, BA>, secret_source: Arc<dyn SecretSource>) -> Self {
        Self {
            swap_communication: SwapCommunication::Proposed { request },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source,
            error: None,
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
            SwapCommunication::Proposed { ref request } => match response {
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
            _ => log::error!("Tried to set a response after it's already set"),
        }
    }

    fn set_secret(&mut self, _secret: Secret) {
        // ignored because Alice already knows the secret
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
