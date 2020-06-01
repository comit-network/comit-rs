use crate::swap_protocols::{halight, herc20, Role, Secret};
use comit::{SecretHash, Timestamp};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct Herc20 {
    pub protocol: String,
    pub quantity: String, // In Wei.
    pub token_contract: String,
}

#[derive(Debug, Serialize)]
pub struct Halight {
    pub protocol: String,
    pub quantity: String, // In Satoshi.
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum EscrowStatus {
    None,
    Initialized,
    Deployed,
    Funded,
    Redeemed,
    Refunded,
    IncorrectlyFunded,
}

pub trait AlphaEvents {
    fn alpha_events(&self) -> Option<LedgerEvents>;
}

pub trait BetaEvents {
    fn beta_events(&self) -> Option<LedgerEvents>;
}

/// Get the underlying blockchain used by the alpha protocol.
pub trait AlphaBlockchain {
    fn alpha_blockchain(&self) -> Blockchain;
}

/// Get the underlying blockchain used by the beta protocol.
pub trait BetaBlockchain {
    fn beta_blockchain(&self) -> Blockchain;
}

/// Get the absolute expiry time for the alpha protocol.
pub trait AlphaAbsoluteExpiry {
    fn alpha_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Get the absolute expiry time for the beta protocol.
pub trait BetaAbsoluteExpiry {
    fn beta_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Blockchains we currently support swaps on top of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Blockchain {
    Bitcoin,
    Ethereum,
}

pub trait GetRole {
    fn get_role(&self) -> Role;
}

pub trait AlphaParams {
    type Output: Serialize;
    fn alpha_params(&self) -> Self::Output;
}

pub trait BetaParams {
    type Output: Serialize;
    fn beta_params(&self) -> Self::Output;
}

#[derive(Debug, Serialize)]
pub struct LedgerEvents {
    /// Keys are on of: "init", "deploy", "fund", "redeem", "refund".
    /// Values are transactions.
    transactions: HashMap<String, String>,
    status: EscrowStatus,
}

impl LedgerEvents {
    fn new(status: EscrowStatus) -> Self {
        Self {
            transactions: HashMap::new(), /* if we want transaction here, we should save the
                                           * events to the DB */
            status,
        }
    }
}

impl From<herc20::State> for LedgerEvents {
    fn from(state: herc20::State) -> Self {
        match state {
            herc20::State::None => LedgerEvents::new(EscrowStatus::None),
            herc20::State::Deployed { .. } => LedgerEvents::new(EscrowStatus::Deployed),
            herc20::State::Funded { .. } => LedgerEvents::new(EscrowStatus::Funded),
            herc20::State::IncorrectlyFunded { .. } => {
                LedgerEvents::new(EscrowStatus::IncorrectlyFunded)
            }
            herc20::State::Redeemed { .. } => LedgerEvents::new(EscrowStatus::Redeemed),
            herc20::State::Refunded { .. } => LedgerEvents::new(EscrowStatus::Refunded),
        }
    }
}

impl From<halight::State> for LedgerEvents {
    fn from(state: halight::State) -> Self {
        match state {
            halight::State::None => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::None,
            },
            halight::State::Opened(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Initialized,
            },
            halight::State::Accepted(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Funded,
            },
            halight::State::Settled(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Redeemed,
            },
            halight::State::Cancelled(_) => LedgerEvents {
                transactions: HashMap::new(),
                status: EscrowStatus::Refunded,
            },
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum AliceSwap<AC, BC, AF, BF> {
    Created {
        alpha_created: AC,
        beta_created: BC,
    },
    Finalized {
        alpha_finalized: AF,
        beta_finalized: BF,
        secret: Secret,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum BobSwap<AC, BC, AF, BF> {
    Created {
        alpha_created: AC,
        beta_created: BC,
    },
    Finalized {
        alpha_finalized: AF,
        beta_finalized: BF,
        secret_hash: SecretHash,
    },
}

impl<AC, BC, AF, BF> GetRole for AliceSwap<AC, BC, AF, BF> {
    fn get_role(&self) -> Role {
        Role::Alice
    }
}

impl<AC, BC, AF, BF> GetRole for BobSwap<AC, BC, AF, BF> {
    fn get_role(&self) -> Role {
        Role::Bob
    }
}
