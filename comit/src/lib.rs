#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::print_stdout,
    clippy::dbg_macro
)]
#![forbid(unsafe_code)]
#![type_length_limit = "1049479"] // Regressed with Rust 1.46.0 :(

pub mod actions;
pub mod asset;
pub mod bitcoin;
pub mod btsieve;
pub mod ethereum;
pub mod expiries;
pub mod halbit;
pub mod hbit;
pub mod herc20;
pub mod htlc_location;
pub mod identity;
pub mod ledger;
pub mod lightning;
pub mod lnd;
pub mod network;
pub mod order;
pub mod orderpool;
#[cfg(test)]
pub mod proptest;
mod secret;
mod secret_hash;
mod timestamp;
pub mod transaction;

/// A module for exporting dependencies that appear in the public API of our
/// crate.
///
/// Ideally, all dependencies of the `comit` crate would be an implementation
/// detail and the consumer doesn't need to worry about their versions for
/// interoperability. However, some types of our dependencies appear in public
/// APIs of the `comit` crate and hence force consumers to use a
/// semver-compatible version of the crate in their application.
///
/// This module allows those consumers to access said dependencies without
/// having to declare a dependency themselves whose version would need to be
/// kept in sync with the one `comit` is depending on.
///
/// Additions to this module should be considered carefully. Removing types
/// defined in dependencies from a public API is almost always preferable over
/// re-exporting the dependency through this module.
pub mod export {
    pub use ::bitcoin;
}

pub use self::{
    network::SharedSwapId,
    order::{BtcDaiOrder, OrderId, Position, Price, Quantity},
    secret::Secret,
    secret_hash::SecretHash,
    timestamp::{RelativeTime, Timestamp},
};

use serde::{Deserialize, Serialize};

/// Defines the set of locking protocol available in COMIT.
///
/// A locking protocol represents a particular way of locking an asset on a
/// certain ledger. Hence, a locking protocol does not only imply the ledger but
/// also the asset that can be locked.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Copy,
    strum_macros::EnumString,
    strum_macros::Display,
    strum_macros::EnumIter,
)]
#[strum(serialize_all = "lowercase")]
pub enum LockProtocol {
    /// The [`hbit`](crate::hbit) locking protocol.
    Hbit,
    /// The [`halbit`](crate::halbit) locking protocol.
    Halbit,
    /// The [`herc20`](crate::herc20) locking protocol.
    Herc20,
}

#[derive(
    Clone,
    Copy,
    Debug,
    strum_macros::Display,
    strum_macros::EnumString,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub enum Role {
    Alice,
    Bob,
}

/// A swap in COMIT is a composition of two protocols, one on each _side_.
///
/// We call those two sides `Alpha` and `Beta` as those are neutral descriptions
/// of the two sides involved. This allows us to talk about the ledgers involved
/// in a swap from a global perspective, i.e. both parties of a swap, Alice and
/// Bob, refer to the same ledger as the `Alpha` ledger. Given knowledge about a
/// party's role and alpha/beta ledger, it is possible to unambiguously describe
/// and observe the actions being taken by either party.
///
/// Taking into account that, by convention, Alice always generated the secret
/// value in COMIT swaps, the following is true:
///
/// In a swap with alpha-ledger = Bitcoin, it is Alice's responsibility to fund
/// a Bitcoin HTLC. In the same swap, it is Bob's responsibility to watch for
/// the funding of this Bitcoin HTLC.
///
/// The terminology of Alpha & Beta is superior to naming schemes like "first -
/// second", "source - target", "buy - sell" etc because it is _global_ and true
/// for both parties. Only the _combination_ of a party's role and the side of a
/// ledger makes it possible to unambiguously reason about the protocol in
/// action.
#[derive(Clone, Copy, Debug, strum_macros::Display, strum_macros::EnumString, PartialEq, Eq)]
pub enum Side {
    Alpha,
    Beta,
}

/// The various networks within COMIT.
#[derive(Debug, Clone, Copy, strum_macros::Display, strum_macros::EnumString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum Network {
    Main,
    Test,
    Dev,
}

impl Default for Network {
    fn default() -> Self {
        Network::Main
    }
}

impl From<Network> for ledger::Bitcoin {
    fn from(network: Network) -> Self {
        match network {
            Network::Main => ledger::Bitcoin::Mainnet,
            Network::Test => ledger::Bitcoin::Testnet,
            Network::Dev => ledger::Bitcoin::Regtest,
        }
    }
}

impl From<Network> for ethereum::ChainId {
    fn from(network: Network) -> Self {
        match network {
            Network::Main => ethereum::ChainId::MAINNET,
            Network::Test => ethereum::ChainId::ROPSTEN,
            Network::Dev => ethereum::ChainId::GETH_DEV,
        }
    }
}

pub type Never = std::convert::Infallible;
