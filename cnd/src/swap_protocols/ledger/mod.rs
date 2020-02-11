pub mod bitcoin;
pub mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};
use crate::swap_protocols::LedgerKind;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, hash::Hash};

// This trait is here to bridge between the old API (http/comit) that
// used to be rfc003 and consider all ledger equals
// once the API is changed to consider han and her20 separately then
// it should hopefully go away
pub trait Ledger:
    Clone + Copy + Debug + Send + Sync + 'static + PartialEq + Eq + Hash + Into<LedgerKind> + Sized
{
    type HtlcLocation: PartialEq + Debug + Clone + DeserializeOwned + Serialize + Send + Sync;
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
