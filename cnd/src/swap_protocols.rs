mod facade;
pub mod halight;
pub mod hbit;
pub mod herc20;
pub mod ledger_states;
pub mod rfc003;
mod rfc003_facade;
pub mod state;
mod swap_error_states;
mod swap_id;
pub mod ledger {
    use crate::comit_api::LedgerKind;
    pub use comit::ledger::*;

    impl From<Bitcoin> for LedgerKind {
        fn from(bitcoin: Bitcoin) -> Self {
            LedgerKind::Bitcoin(bitcoin)
        }
    }

    impl From<Ethereum> for LedgerKind {
        fn from(ethereum: Ethereum) -> Self {
            LedgerKind::Ethereum(ethereum)
        }
    }
}
pub mod actions {
    /// Common interface across all protocols supported by COMIT
    ///
    /// This trait is intended to be implemented on an Actor's state and return
    /// the actions which are currently available in a given state.
    pub trait Actions {
        /// Different protocols have different kinds of requirements for
        /// actions. Hence they get to choose the type here.
        type ActionKind;

        fn actions(&self) -> Vec<Self::ActionKind>;
    }

    pub use comit::actions::*;
}

pub use self::{facade::*, ledger_states::*, rfc003_facade::*, swap_error_states::*, swap_id::*};
pub use comit::{Secret, SecretHash};

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Copy,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
pub enum HashFunction {
    #[strum(serialize = "SHA-256")]
    #[serde(rename = "SHA-256")]
    Sha256,
}

#[derive(Debug, Clone, Copy)]
pub enum SwapProtocol {
    Rfc003(HashFunction),
}

#[derive(Clone, Copy, Debug, Display, EnumString, PartialEq)]
pub enum Ledger {
    Alpha,
    Beta,
}

#[derive(Clone, Copy, Debug, Display, EnumString, PartialEq)]
pub enum Role {
    Alice,
    Bob,
}

/// These are the traits that represent the steps involved in a COMIT atomic
/// swap.  Different protocols have different requirements/functionality for
/// each trait method but the abstractions are the same for all protocols.

/// Describes how to get the `init` action from the current state.
///
/// If `init` is not feasible in the current state, this should return `None`.
pub trait InitAction {
    type Output;

    fn init_action(&self) -> Option<Self::Output>;
}

/// Describes how to get the `fund` action from the current state.
///
/// If `fund` is not feasible in the current state, this should return `None`.
pub trait FundAction {
    type Output;

    fn fund_action(&self) -> Option<Self::Output>;
}

pub trait DeployAction {
    type Output;

    fn deploy_action(&self) -> Option<Self::Output>;
}

/// Describes how to get the `redeem` action from the current state.
///
/// If `redeem` is not feasible in the current state, this should return `None`.
pub trait RedeemAction {
    type Output;

    fn redeem_action(&self) -> Option<Self::Output>;
}

/// Describes how to get the `refund` action from the current state.
///
/// If `refund` is not feasible in the current state, this should return `None`.
pub trait RefundAction {
    type Output;

    fn refund_action(&self) -> Option<Self::Output>;
}
