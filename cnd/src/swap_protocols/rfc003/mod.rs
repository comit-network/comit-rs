#[macro_use]
mod transition_save;

pub mod alice;
pub mod bitcoin;
pub mod bob;
pub mod ethereum;
pub mod events;
pub mod ledger_state;
pub mod messages;
pub mod spawn;
pub mod state_machine;
pub mod state_store;

pub mod actions;
mod actor_state;
mod ledger;
mod save_state;
mod secret;
mod secret_source;

pub mod create_ledger_events;

pub use self::{
    actor_state::ActorState,
    create_ledger_events::CreateLedgerEvents,
    ledger::Ledger,
    ledger_state::{HtlcState, LedgerState},
    save_state::SaveState,
    secret::{FromErr, Secret, SecretHash},
    secret_source::*,
    spawn::*,
};

pub use self::messages::{Accept, Decline, Request};
/// Swap request response as received from peer node acting as Bob.
pub type Response<AL, BL> = Result<Accept<AL, BL>, Decline>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Btsieve,
    TimerError,
    IncorrectFunding,
    Internal(String),
}
