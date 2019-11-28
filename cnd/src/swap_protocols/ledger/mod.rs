mod bitcoin;
pub mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};

use crate::btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use derivative::Derivative;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, hash::Hash};

pub trait Ledger:
    Clone + Copy + Debug + Send + Sync + 'static + Default + PartialEq + Eq + Hash + Into<LedgerKind>
{
    type Identity: Clone
        + Copy
        + Debug
        + Send
        + Sync
        + PartialEq
        + Eq
        + Hash
        + 'static
        + Serialize
        + DeserializeOwned;
    type Transaction: Debug
        + Clone
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + PartialEq
        + 'static;
}

#[derive(Clone, Derivative, PartialEq)]
#[derivative(Debug = "transparent")]
pub enum LedgerKind {
    Bitcoin(Bitcoin),
    Ethereum(Ethereum),
    Unknown(String),
}

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct LedgerConnectors {
    pub bitcoin_connector: BitcoindConnector,
    pub ethereum_connector: Web3Connector,
}
