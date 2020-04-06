pub mod actions;
mod facade;
mod facade2;
pub mod halight;
pub mod han;
pub mod ledger;
pub mod ledger_states;
pub mod rfc003;
pub mod state;
pub mod swap_communication_states;
mod swap_error_states;
mod swap_id;

pub use self::{
    facade::*, facade2::*, ledger_states::*, swap_communication_states::*, swap_error_states::*,
    swap_id::*,
};

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
