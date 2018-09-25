use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap;

pub trait Ledger: Clone + Debug + Send + Sync + 'static + Default + Into<swap::Ledger> {
    type Quantity: Debug + Copy + DeserializeOwned + Serialize + Send + Sync + 'static;
    type LockDuration: Debug
        + Clone
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + Default
        + 'static;
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
    type TxId: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Pubkey: Clone + Debug + Send + Sync + 'static;
    type Address: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Identity: Clone
        + Debug
        + Send
        + Sync
        + Default
        + 'static
        + From<Self::Address>
        + Serialize
        + DeserializeOwned;

    fn symbol() -> String;
    fn address_for_identity(&self, Self::Identity) -> Self::Address;
}

pub mod bitcoin;
pub mod ethereum;
