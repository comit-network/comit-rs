use crate::{halbit, hbit, herc20, Role, Secret, SecretHash, Timestamp};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct Herc20 {
    pub protocol: String,
    pub quantity: String, // In Wei.
    pub token_contract: String,
}

#[derive(Debug, Serialize)]
pub struct Halbit {
    pub protocol: String,
    pub quantity: String, // In Satoshi.
}

#[derive(Debug, Serialize)]
pub struct Hbit {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActionName {
    Init,
    Deploy,
    Fund,
    Redeem,
    Refund,
}

pub trait AlphaEvents {
    fn alpha_events(&self) -> Option<LedgerEvents>;
}

pub trait BetaEvents {
    fn beta_events(&self) -> Option<LedgerEvents>;
}

/// Get the underlying ledger used by the alpha protocol.
pub trait AlphaLedger {
    fn alpha_ledger(&self) -> Ledger;
}

/// Get the underlying ledger used by the beta protocol.
pub trait BetaLedger {
    fn beta_ledger(&self) -> Ledger;
}

/// Get the absolute expiry time for the alpha protocol.
pub trait AlphaAbsoluteExpiry {
    fn alpha_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Get the absolute expiry time for the beta protocol.
pub trait BetaAbsoluteExpiry {
    fn beta_absolute_expiry(&self) -> Option<Timestamp>;
}

/// Ledgers we currently support swaps on top of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ledger {
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
    events: HashMap<ActionName, String>,
    status: EscrowStatus,
}

impl LedgerEvents {
    fn new(status: EscrowStatus, events: HashMap<ActionName, String>) -> Self {
        Self { events, status }
    }
}

impl From<herc20::State> for LedgerEvents {
    fn from(state: herc20::State) -> Self {
        match state {
            herc20::State::None => LedgerEvents::new(EscrowStatus::None, HashMap::new()),
            herc20::State::Deployed {
                deploy_transaction, ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Deploy, format!("{}", deploy_transaction.hash));
                LedgerEvents::new(EscrowStatus::Deployed, transactions)
            }
            herc20::State::Funded {
                deploy_transaction,
                fund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Deploy, format!("{}", deploy_transaction.hash));
                transactions.insert(ActionName::Fund, format!("{}", fund_transaction.hash));
                LedgerEvents::new(EscrowStatus::Funded, transactions)
            }
            herc20::State::IncorrectlyFunded {
                deploy_transaction,
                fund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Deploy, format!("{}", deploy_transaction.hash));
                transactions.insert(ActionName::Fund, format!("{}", fund_transaction.hash));
                LedgerEvents::new(EscrowStatus::IncorrectlyFunded, transactions)
            }
            herc20::State::Redeemed {
                deploy_transaction,
                fund_transaction,
                redeem_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Deploy, format!("{}", deploy_transaction.hash));
                transactions.insert(ActionName::Fund, format!("{}", fund_transaction.hash));
                transactions.insert(ActionName::Redeem, format!("{}", redeem_transaction.hash));
                LedgerEvents::new(EscrowStatus::Redeemed, transactions)
            }
            herc20::State::Refunded {
                deploy_transaction,
                fund_transaction,
                refund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Deploy, format!("{}", deploy_transaction.hash));
                transactions.insert(ActionName::Fund, format!("{}", fund_transaction.hash));
                transactions.insert(ActionName::Refund, format!("{}", refund_transaction.hash));
                LedgerEvents::new(EscrowStatus::Refunded, transactions)
            }
        }
    }
}

impl From<hbit::State> for LedgerEvents {
    fn from(state: hbit::State) -> Self {
        match state {
            hbit::State::None => LedgerEvents::new(EscrowStatus::None, HashMap::new()),
            hbit::State::Funded {
                fund_transaction, ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Fund, fund_transaction.txid().to_string());
                LedgerEvents::new(EscrowStatus::Funded, transactions)
            }
            hbit::State::IncorrectlyFunded {
                fund_transaction, ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Fund, fund_transaction.txid().to_string());
                LedgerEvents::new(EscrowStatus::IncorrectlyFunded, transactions)
            }
            hbit::State::Redeemed {
                fund_transaction,
                redeem_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Fund, fund_transaction.txid().to_string());
                transactions.insert(ActionName::Redeem, redeem_transaction.txid().to_string());
                LedgerEvents::new(EscrowStatus::Redeemed, transactions)
            }
            hbit::State::Refunded {
                fund_transaction,
                refund_transaction,
                ..
            } => {
                let mut transactions = HashMap::new();
                transactions.insert(ActionName::Fund, fund_transaction.txid().to_string());
                transactions.insert(ActionName::Refund, refund_transaction.txid().to_string());
                LedgerEvents::new(EscrowStatus::Refunded, transactions)
            }
        }
    }
}

impl From<halbit::State> for LedgerEvents {
    fn from(state: halbit::State) -> Self {
        match state {
            halbit::State::None => LedgerEvents::new(EscrowStatus::None, HashMap::new()),
            halbit::State::Opened(_) => {
                LedgerEvents::new(EscrowStatus::Initialized, HashMap::new())
            }
            halbit::State::Accepted(_) => LedgerEvents::new(EscrowStatus::Funded, HashMap::new()),
            halbit::State::Settled(_) => LedgerEvents::new(EscrowStatus::Redeemed, HashMap::new()),
            halbit::State::Cancelled(_) => {
                LedgerEvents::new(EscrowStatus::Refunded, HashMap::new())
            }
        }
    }
}

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
