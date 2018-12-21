#[macro_use]
mod transition_save;

pub mod actions;
pub mod alice;
pub mod bitcoin;
pub mod bob;
pub mod ethereum;
pub mod events;
pub mod find_htlc_location;
pub mod roles;

pub mod state_machine;
pub mod state_store;

mod error;

mod ledger;
mod save_state;
mod secret;
mod secret_source;

#[cfg(test)]
mod state_machine_test;

pub use self::{
    error::Error,
    ledger::{ExtractSecret, FundTransaction, Ledger, RedeemTransaction, RefundTransaction},
    save_state::SaveState,
    secret::{FromErr, RandomnessSource, Secret, SecretHash},
    secret_source::*,
};
