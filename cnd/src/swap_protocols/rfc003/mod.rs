#[macro_use]
mod transition_save;

pub mod alice;
pub mod bitcoin;
pub mod bob;
pub mod ethereum;
pub mod events;
pub mod ledger_state;
pub mod messages;
pub mod state_machine;
pub mod state_store;

pub mod actions;
mod actor_state;
mod error;
pub mod insert_state;
mod ledger;
mod save_state;
mod secret;
mod secret_source;

pub mod create_ledger_events;

pub use self::{
    actor_state::ActorState,
    create_ledger_events::CreateLedgerEvents,
    error::Error,
    insert_state::InsertState,
    ledger::Ledger,
    ledger_state::{HtlcState, LedgerState},
    messages::*,
    save_state::SaveState,
    secret::{FromErr, Secret, SecretHash},
    secret_source::*,
};

use self::messages::{AcceptResponseBody, DeclineResponseBody};

/// Swap request response as received from peer node acting as Bob.
pub type Response<AL, BL> = Result<AcceptResponseBody<AL, BL>, DeclineResponseBody>;
