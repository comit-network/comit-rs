pub mod oneshot_behaviour;
pub mod oneshot_protocol;
pub mod protocols;

use crate::{identity, SecretHash};
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

/// All the data Alice learned from Bob during the communication phase of a
/// herc20 <-> halight swap.
#[derive(Clone, Copy, Debug)]
pub struct WhatAliceLearnedFromBob {
    pub redeem_ethereum_identity: identity::Ethereum,
    pub refund_lightning_identity: identity::Lightning,
}

/// All the data Bob learned from Alice during the communication phase of a
/// herc20 <-> halight swap.
#[derive(Clone, Copy, Debug)]
pub struct WhatBobLearnedFromAlice {
    pub secret_hash: SecretHash,
    pub refund_ethereum_identity: identity::Ethereum,
    pub redeem_lightning_identity: identity::Lightning,
}
