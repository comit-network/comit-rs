use ledger_query_service::BitcoinQuery;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols;

pub trait Ledger: swap_protocols::ledger::Ledger {
    type LockDuration: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static;
    //TODO: Rename "ContractLocation"
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
    type HtlcIdentity: Clone
        + Send
        + Sync
        + Into<<Self as swap_protocols::ledger::Ledger>::Identity>;
}
