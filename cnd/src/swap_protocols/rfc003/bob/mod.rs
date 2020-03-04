pub mod actions;

use crate::swap_protocols::rfc003::{
    ledger::Ledger, ledger_state::LedgerState, messages::Request, Accept, ActorState, Decline,
    DeriveIdentities, SwapCommunication,
};
use derivative::Derivative;
use std::sync::Arc;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct State<AL, BL, AA, BA, AH, BH, AI, BI>
where
    AL: Ledger,
    BL: Ledger,
{
    pub swap_communication: SwapCommunication<AL, BL, AA, BA, AI, BI>,
    pub alpha_ledger_state: LedgerState<AA, AH, AL::Transaction>,
    pub beta_ledger_state: LedgerState<BA, BH, BL::Transaction>,
    #[derivative(Debug = "ignore")]
    pub secret_source: Arc<dyn DeriveIdentities>,
    pub failed: bool, // Gets set on any error during the execution of a swap.
}

impl<AL, BL, AA, BA, AH, BH, AI, BI> State<AL, BL, AA, BA, AH, BH, AI, BI>
where
    AL: Ledger,
    BL: Ledger,
{
    pub fn proposed(
        request: Request<AL, BL, AA, BA, AI, BI>,
        secret_source: impl DeriveIdentities,
    ) -> Self {
        Self {
            swap_communication: SwapCommunication::Proposed { request },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source: Arc::new(secret_source),
            failed: false,
        }
    }

    pub fn accepted(
        request: Request<AL, BL, AA, BA, AI, BI>,
        response: Accept<AI, BI>,
        secret_source: impl DeriveIdentities,
    ) -> Self {
        Self {
            swap_communication: SwapCommunication::Accepted { request, response },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source: Arc::new(secret_source),
            failed: false,
        }
    }

    pub fn declined(
        request: Request<AL, BL, AA, BA, AI, BI>,
        response: Decline,
        secret_source: impl DeriveIdentities,
    ) -> Self {
        Self {
            swap_communication: SwapCommunication::Declined { request, response },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source: Arc::new(secret_source),
            failed: false,
        }
    }

    pub fn request(&self) -> &Request<AL, BL, AA, BA, AI, BI> {
        match &self.swap_communication {
            SwapCommunication::Accepted { request, .. }
            | SwapCommunication::Proposed { request, .. }
            | SwapCommunication::Declined { request, .. } => request,
        }
    }
}

impl<AL, BL, AA, BA, AH, BH, AI, BI> ActorState for State<AL, BL, AA, BA, AH, BH, AI, BI>
where
    AL: Ledger,
    BL: Ledger,
    AA: 'static,
    BA: 'static,
    AI: 'static,
    BI: 'static,
    AH: 'static,
    BH: 'static,
{
    type AL = AL;
    type BL = BL;
    type AA = AA;
    type BA = BA;
    type AH = AH;
    type BH = BH;

    fn expected_alpha_asset(&self) -> &Self::AA {
        &self.swap_communication.request().alpha_asset
    }

    fn expected_beta_asset(&self) -> &Self::BA {
        &self.swap_communication.request().beta_asset
    }

    fn alpha_ledger_mut(&mut self) -> &mut LedgerState<AA, AH, AL::Transaction> {
        &mut self.alpha_ledger_state
    }

    fn beta_ledger_mut(&mut self) -> &mut LedgerState<BA, BH, BL::Transaction> {
        &mut self.beta_ledger_state
    }

    fn swap_failed(&self) -> bool {
        self.failed
    }

    fn set_swap_failed(&mut self) {
        self.failed = true;
    }
}
