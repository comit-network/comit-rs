use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

pub trait Ledger: Clone + Debug + Send + Sync + 'static + Default {
    type Quantity: Debug + Copy + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Address: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type LockDuration: Debug + Clone + Send + Sync + 'static;
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
    type TxId: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Pubkey: Clone + Debug + Send + Sync + 'static;
    type Identity: Clone + Debug + Send + Sync + 'static;

    fn symbol() -> String;
}

pub mod bitcoin;
pub mod ethereum;
