//! Recommended Safe Actions
//!
//! This module provides logic for recommending when it is safe/unsafe to take a
//! particular action for the COMIT protocol.

//! 'Safe' is defined as:
//! 1 - You will not loose money.
//! 2 - The counterparty is likely to continue with the swap because it is safe
//!     for them to do so.

use crate::Timestamp;
use time::Duration;

/// SwapTime enables recommendations on whether it is safe to take an action.
#[derive(Debug, Clone, Copy)]
pub struct SwapTime {
    alpha_ledger: Ledger,
    beta_ledger: Ledger,
    alpha_expiry: Timestamp,
    beta_expiry: Timestamp,
}

impl SwapTime {
    /// Construct a new SwapTime from the given expiries.
    pub fn new(
        alpha_ledger: Ledger,
        beta_ledger: Ledger,
        alpha_expiry: Timestamp,
        beta_expiry: Timestamp,
    ) -> Self {
        SwapTime {
            alpha_ledger,
            beta_ledger,
            alpha_expiry,
            beta_expiry,
        }
    }

    /// Calculates a pair of valid expiries using the given parameters.
    /// The created SwapTime will return true from calls to
    /// is_time_for_swap_to_complete() for the duration of `good_for`.
    pub fn with_calculated_expiries(
        _alpha_ledger: Ledger,
        _beta_ledger: Ledger,
        _good_for: Duration,
    ) -> Result<Self, InsufficientTime> {
        unimplemented!()
    }

    /// Helper method so we do not need to know which action is first (since
    /// that is ledger specific).
    pub fn is_safe_for_alice_to_start(&self) -> bool {
        match self.alpha_ledger {
            Ledger::Bitcoin => self.is_safe_for_alice_to_fund(),
            Ledger::Ethereum => self.is_safe_for_alice_to_deploy(),
        }
    }

    /// Returns true if is safe for Alice to initialise the swap now.
    pub fn is_safe_for_alice_to_init(&self) -> bool {
        // TODO: Alice init
        true
    }

    /// Returns true if is safe for Alice to deploy the swap now.
    pub fn is_safe_for_alice_to_deploy(&self) -> bool {
        // TODO: Alice deploy
        true
    }

    /// Returns true if is safe for Alice to fund the swap now.
    pub fn is_safe_for_alice_to_fund(&self) -> bool {
        // TODO: Alice fund
        true
    }

    /// Returns true if is safe for Alice to redeem the swap now.
    pub fn is_safe_for_alice_to_redeem(&self) -> bool {
        // TODO: Alice redeem
        true
    }
    /// Returns true if is safe for Alice to refund the swap now.
    pub fn is_safe_for_alice_to_refund(&self) -> bool {
        // It is always safe to refund.
        true
    }

    /// Returns true if is safe for Bob to initialise the swap now.
    pub fn is_safe_for_bob_to_init(&self) -> bool {
        // TODO: Bob init
        true
    }

    /// Returns true if is safe for Bob to deploy the swap now.
    pub fn is_safe_for_bob_to_deploy(&self) -> bool {
        // TODO: Bob deploy
        true
    }

    /// Returns true if is safe for Bob to fund the swap now.
    pub fn is_safe_for_bob_to_fund(&self) -> bool {
        // TODO: Bob fund
        true
    }

    /// Returns true if is safe for Bob to redeem the swap now.
    pub fn is_safe_for_bob_to_redeem(&self) -> bool {
        // It is always safe for Bob to redeem.
        true
    }

    /// Returns true if is safe for Bob to refund the swap now.
    pub fn is_safe_for_bob_to_refund(&self) -> bool {
        // It is always safe to refund.
        true
    }
}

/// Supported ledgers.
#[derive(Debug, Clone, Copy)]
pub enum Ledger {
    // TODO: Add support for LND
    Bitcoin,
    Ethereum,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Insufficient time to safely take all actions")]
pub struct InsufficientTime;
