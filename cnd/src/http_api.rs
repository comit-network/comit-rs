mod action;
mod amount;
mod dial_addr;
pub mod halbit;
mod halbit_herc20;
pub mod hbit;
mod hbit_herc20;
pub mod herc20;
mod herc20_halbit;
mod herc20_hbit;
mod info;
mod markets;
mod orders;
mod peers;
mod problem;
mod protocol;
mod route_factory;
mod serde_peer_id;
mod swaps;
mod tokens;

pub use self::{
    halbit::Halbit,
    hbit::Hbit,
    herc20::Herc20,
    problem::*,
    protocol::{AliceSwap, BobSwap},
    route_factory::create as create_routes,
};

pub const PATH: &str = "swaps";

use crate::{storage::CreatedSwap, LocalSwapId, Role};
use chrono::Utc;
use libp2p::{Multiaddr, PeerId};
use serde::Deserialize;

/// Object representing the data of a POST request for creating a swap.
#[derive(Deserialize, Clone, Debug)]
pub struct PostBody<A, B> {
    pub alpha: A,
    pub beta: B,
    pub peer: DialInformation,
    pub role: Role,
}

impl<A, B> PostBody<A, B> {
    pub fn to_created_swap<CA, CB>(&self, swap_id: LocalSwapId) -> CreatedSwap<CA, CB>
    where
        CA: From<A>,
        CB: From<B>,
        Self: Clone,
    {
        let body = self.clone();

        let alpha = CA::from(body.alpha);
        let beta = CB::from(body.beta);

        let start_of_swap = Utc::now().naive_local();

        CreatedSwap {
            swap_id,
            alpha,
            beta,
            peer: body.peer.into(),
            address_hint: None,
            role: body.role,
            start_of_swap,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DialInformation {
    JustPeerId(#[serde(with = "serde_peer_id")] PeerId),
    WithAddressHint {
        #[serde(with = "serde_peer_id")]
        peer_id: PeerId,
        address_hint: Multiaddr,
    },
}

impl DialInformation {
    fn into_peer_with_address_hint(self) -> (PeerId, Option<Multiaddr>) {
        match self {
            DialInformation::JustPeerId(inner) => (inner, None),
            DialInformation::WithAddressHint {
                peer_id,
                address_hint,
            } => (peer_id, Some(address_hint)),
        }
    }
}

impl From<DialInformation> for PeerId {
    fn from(dial_information: DialInformation) -> Self {
        match dial_information {
            DialInformation::JustPeerId(inner) => inner,
            DialInformation::WithAddressHint { peer_id, .. } => peer_id,
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("action not found")]
pub struct ActionNotFound;
