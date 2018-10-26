use ledger_query_service::BitcoinQuery;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols;

pub trait Ledger: swap_protocols::ledger::Ledger {
    type LockDuration: PartialEq
        + Debug
        + Clone
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + 'static;
    //TODO: Rename "ContractLocation"
    type HtlcId: PartialEq + Debug + Clone + DeserializeOwned + Serialize + Send + Sync;
    type HtlcIdentity: Clone
        + Send
        + Sync
        + PartialEq
        + Debug
        + Into<<Self as swap_protocols::ledger::Ledger>::Identity>;
}
