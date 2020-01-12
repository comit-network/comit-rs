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

pub use self::{
    actor_state::ActorState,
    create_swap::create_swap,
    ledger::Ledger,
    ledger_state::{HtlcState, LedgerState},
    secret::{FromErr, Secret, SecretHash},
};

pub use self::messages::{Accept, Decline, Request};

use crate::{seed::SwapSeed, swap_protocols::asset::Asset};
use ::bitcoin::secp256k1::SecretKey;

/// Swap request response as received from peer node acting as Bob.
pub type Response<AL, BL> = Result<Accept<AL, BL>, Decline>;

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

pub trait DeriveIdentities: Send + Sync + 'static {
    fn derive_redeem_identity(&self) -> SecretKey;
    fn derive_refund_identity(&self) -> SecretKey;
}

/// Both Alice and Bob use their `SwapSeed` to derive identities.
impl DeriveIdentities for SwapSeed {
    fn derive_redeem_identity(&self) -> SecretKey {
        SecretKey::from_slice(self.sha256_with_seed(&[b"REDEEM"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }

    fn derive_refund_identity(&self) -> SecretKey {
        SecretKey::from_slice(self.sha256_with_seed(&[b"REFUND"]).as_ref())
            .expect("The probability of this happening is < 1 in 2^120")
    }
}

pub trait DeriveSecret: Send + Sync + 'static {
    fn derive_secret(&self) -> Secret;
}

/// Only Alice derives the secret, Bob learns the secret from Alice.
impl DeriveSecret for SwapSeed {
    fn derive_secret(&self) -> Secret {
        self.sha256_with_seed(&[b"SECRET"]).into()
    }
}
