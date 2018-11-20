#[macro_use]
mod transition_save;

pub mod actions;
pub mod alice;
pub mod alice_ledger_actor;
pub mod bitcoin;
pub mod bob;
pub mod ethereum;
pub mod events;
pub mod is_contained_in_transaction;
pub mod ledger_htlc_service;
pub mod roles;
pub mod state_machine;
pub mod state_store;

mod error;

mod ledger;
mod outcome;
mod save_state;
mod secret;

#[cfg(test)]
mod state_machine_test;

pub use self::{
    error::Error,
    ledger::{ExtractSecret, Ledger},
    outcome::SwapOutcome,
    save_state::SaveState,
    secret::{RandomnessSource, Secret, SecretFromErr, SecretHash},
};
