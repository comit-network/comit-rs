pub mod comit;
pub mod oneshot_behaviour;
pub mod oneshot_protocol;
pub mod orderbook;
pub mod protocols;
mod shared_swap_id;
pub mod swap_digest;
#[cfg(any(test, feature = "test"))]
pub mod test;

use crate::{identity, SecretHash};

pub use self::{
    comit::*,
    orderbook::*,
    protocols::{announce::Announce, *},
    shared_swap_id::SharedSwapId,
    swap_digest::SwapDigest,
};

/// All possible identities to be sent to the remote node for any protocol
/// combination.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Identities {
    pub ethereum_identity: Option<identity::Ethereum>,
    pub lightning_identity: Option<identity::Lightning>,
    pub bitcoin_identity: Option<identity::Bitcoin>,
}

/// Data Alice learned from Bob during the communication phase.
#[derive(Clone, Copy, Debug)]
pub struct WhatAliceLearnedFromBob<A, B> {
    pub alpha_redeem_identity: A,
    pub beta_refund_identity: B,
}

/// Data Bob learned from Alice during the communication phase.
#[derive(Clone, Copy, Debug)]
pub struct WhatBobLearnedFromAlice<A, B> {
    pub secret_hash: SecretHash,
    pub alpha_refund_identity: A,
    pub beta_redeem_identity: B,
}
