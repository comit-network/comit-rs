pub mod oneshot_behaviour;
pub mod oneshot_protocol;
pub mod protocols;
pub mod swap_digest;

use crate::SecretHash;
use libp2p::{Multiaddr, PeerId};
use std::fmt;

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

/// Data Alice learned from Bob during the communication phase.
#[derive(Clone, Copy, Debug)]
pub struct WhatAliceLearnedFromBob<A, B> {
    pub redeem_identity: A,
    pub refund_identity: B,
}

/// Data Bob learned from Alice during the communication phase.
#[derive(Clone, Copy, Debug)]
pub struct WhatBobLearnedFromAlice<A, B> {
    pub secret_hash: SecretHash,
    pub refund_identity: A,
    pub redeem_identity: B,
}
