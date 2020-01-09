#[macro_use]
mod transition_save;

pub mod alice;
pub mod bitcoin;
pub mod bob;
pub mod create_swap;
pub mod ethereum;
pub mod events;
pub mod ledger_state;
pub mod messages;
pub mod state_store;

pub mod actions;
mod actor_state;
mod ledger;
mod secret;
mod secret_source;

pub use self::{
    actor_state::ActorState,
    create_swap::create_swap,
    ledger::Ledger,
    ledger_state::{HtlcState, LedgerState},
    secret::{FromErr, Secret, SecretHash},
    secret_source::*,
};

pub use self::messages::{Accept, Decline, Request};

use crate::swap_protocols::asset::Asset;

/// Swap request response as received from peer node acting as Bob.
pub type Response<AL, BL> = Result<Accept<AL, BL>, Decline>;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("btsieve")]
    Btsieve,
    #[error("timer error")]
    TimerError,
    #[error("incorrect funding")]
    IncorrectFunding,
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SwapCommunication<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    Proposed {
        request: Request<AL, BL, AA, BA>,
    },
    Accepted {
        request: Request<AL, BL, AA, BA>,
        response: Accept<AL, BL>,
    },
    Declined {
        request: Request<AL, BL, AA, BA>,
        response: Decline,
    },
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> SwapCommunication<AL, BL, AA, BA> {
    pub fn request(&self) -> &Request<AL, BL, AA, BA> {
        match self {
            SwapCommunication::Accepted { request, .. } => request,
            SwapCommunication::Proposed { request } => request,
            SwapCommunication::Declined { request, .. } => request,
        }
    }
}
