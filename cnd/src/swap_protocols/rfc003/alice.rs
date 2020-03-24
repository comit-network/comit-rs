mod actions;

pub use self::actions::*;

use crate::{
    seed::SwapSeed,
    swap_protocols::rfc003::{ledger_state::LedgerState, messages, SwapCommunication},
};
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq)]
pub struct State<AL, BL, AA, BA, AH, BH, AI, BI, AT, BT> {
    pub swap_communication: SwapCommunication<AL, BL, AA, BA, AI, BI>,
    pub alpha_ledger_state: LedgerState<AA, AH, AT>,
    pub beta_ledger_state: LedgerState<BA, BH, BT>,
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    pub secret_source: SwapSeed, // Used to derive identities and also to generate the secret.
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
            secret_source,
        }
    }

    pub fn request(&self) -> &messages::Request<AL, BL, AA, BA, AI, BI> {
        self.swap_communication.request()
    }
}
