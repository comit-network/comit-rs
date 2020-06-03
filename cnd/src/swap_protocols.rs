pub mod ledger_states;
pub mod rfc003;
mod rfc003_facade;
pub mod state;
mod swap_error_states;
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

pub use self::{ledger_states::*, rfc003_facade::*, swap_error_states::*};

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
