use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols::{self, rfc003::secret::LedgerExtractSecret};

pub trait Ledger: LedgerExtractSecret {
    type LockDuration: PartialEq
        + Debug
        + Clone
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + 'static;
    type HtlcLocation: PartialEq + Debug + Clone + DeserializeOwned + Serialize + Send + Sync;
    type HtlcIdentity: Clone
        + Send
        + Sync
        + PartialEq
        + Debug
        + Into<<Self as swap_protocols::ledger::Ledger>::Identity>;
}
