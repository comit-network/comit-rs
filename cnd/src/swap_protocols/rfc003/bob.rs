pub mod actions;

use crate::{
    seed::SwapSeed,
    swap_protocols::rfc003::{
        ledger_state::LedgerState, messages::Request, DeriveIdentities, SwapCommunication,
    },
};
use derivative::Derivative;
use std::sync::Arc;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT> {
    pub swap_communication: SwapCommunication<AL, BL, AA, BA, AI, BI>,
    pub alpha_ledger_state: LedgerState<AA, AH, AT>,
    pub beta_ledger_state: LedgerState<BA, BH, BT>,
    #[derivative(Debug = "ignore")]
    pub secret_source: Arc<dyn DeriveIdentities>,
}

impl<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT> State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT> {
    pub fn new(
        swap_communication: SwapCommunication<AL, BL, AA, BA, AI, BI>,
        alpha_ledger_state: LedgerState<AA, AH, AT>,
        beta_ledger_state: LedgerState<BA, BH, BT>,
        secret_source: SwapSeed,
    ) -> Self {
        Self {
            swap_communication,
            alpha_ledger_state,
            beta_ledger_state,
            secret_source: Arc::new(secret_source),
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
