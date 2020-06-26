pub mod execution_parameters;
pub mod oneshot_behaviour;
pub mod oneshot_protocol;
pub mod protocols;
pub mod swap_digest;
#[cfg(test)]
pub mod test_swarm;

use crate::{identity, SecretHash};
use libp2p::{Multiaddr, PeerId};
use std::fmt;

pub use self::{
    execution_parameters::*,
    protocols::{announce::Announce, *},
    swap_digest::SwapDigest,
};

#[derive(Clone, Debug, PartialEq)]
pub struct DialInformation {
    pub peer_id: PeerId,
    pub address_hint: Option<Multiaddr>,
}

impl fmt::Display for DialInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match &self.address_hint {
            None => write!(f, "{}", self.peer_id),
            Some(address_hint) => write!(f, "{}@{}", self.peer_id, address_hint),
        }
    }
}

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
