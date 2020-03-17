pub mod alice;
pub mod bitcoin;
pub mod bob;
pub mod create_swap;
pub mod ethereum;
pub mod events;
pub mod ledger_state;
pub mod messages;

pub mod actions;
mod secret;

pub use self::{
    create_swap::create_watcher,
    ledger_state::{HtlcState, LedgerState},
    secret::{FromErr, Secret, SecretHash},
};

pub use self::messages::{Accept, Decline, Request};

use crate::seed::SwapSeed;
use ::bitcoin::secp256k1::SecretKey;

/// Swap request response as received from peer node acting as Bob.
pub type Response<AI, BI> = Result<Accept<AI, BI>, Decline>;

#[derive(Clone, Debug, PartialEq)]
pub enum SwapCommunication<AL, BL, AA, BA, AI, BI> {
    Proposed {
        request: Request<AL, BL, AA, BA, AI, BI>,
    },
    Accepted {
        request: Request<AL, BL, AA, BA, AI, BI>,
        response: Accept<AI, BI>,
    },
    Declined {
        request: Request<AL, BL, AA, BA, AI, BI>,
        response: Decline,
    },
}

impl<AL, BL, AA, BA, AI, BI> SwapCommunication<AL, BL, AA, BA, AI, BI> {
    pub fn request(&self) -> &Request<AL, BL, AA, BA, AI, BI> {
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
