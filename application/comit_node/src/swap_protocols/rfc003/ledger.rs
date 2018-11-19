use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols::{
    self,
    rfc003::secret::{Secret, SecretHash},
};

pub trait Ledger: swap_protocols::Ledger + ExtractSecret {
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

pub trait ExtractSecret: swap_protocols::ledger::Ledger {
    fn extract_secret(txn: &Self::Transaction, secret_hash: &SecretHash) -> Option<Secret>;
}
