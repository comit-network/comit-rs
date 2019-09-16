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
mod ledger;
mod save_state;
mod secret;
mod secret_source;

mod create_ledger_events;

pub use self::{
    actor_state::ActorState,
    create_ledger_events::CreateLedgerEvents,
    error::Error,
    ledger::Ledger,
    ledger_state::{HtlcState, LedgerState},
    save_state::SaveState,
    secret::{FromErr, Secret, SecretHash},
    secret_source::*,
};

use crypto::{digest::Digest, sha2::Sha256};

/// Generates a deterministic identifier from the secret hash and prefix.
/// Returns SHA-256(prefix + secret_hash).
pub fn generate_identifier(secret_hash: &SecretHash, prefix: &str) -> String {
    let mut msg = String::from(prefix);
    msg.push_str(&secret_hash.to_string());
    hash(&msg)
}

fn hash(msg: &str) -> String {
    let mut sha = Sha256::new();
    sha.input_str(msg);
    sha.result_str()
}
