use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

pub trait Ledger: crate::Ledger {
    type HtlcLocation: PartialEq + Debug + Clone + DeserializeOwned + Serialize + Send + Sync;
}
