use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols;

pub trait Ledger: swap_protocols::ledger::Ledger {
    type LockDuration: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static;
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
}
