use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, hash::Hash};

pub trait Ledger:
    Clone + Copy + Debug + Send + Sync + 'static + PartialEq + Eq + Hash + Sized
{
    type Transaction: Debug
        + Clone
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + PartialEq
        + 'static;
}
