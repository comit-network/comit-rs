use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols;

pub trait Ledger:
    Clone + Debug + Send + Sync + 'static + Default + Into<swap_protocols::wire_types::Ledger>
{
    type Quantity: Debug + Copy + DeserializeOwned + Serialize + Send + Sync + 'static;
    type LockDuration: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static;
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
    type TxId: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Pubkey: Clone + Debug + Send + Sync + 'static;
    type Address: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Identity: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + From<Self::Address>
        + Into<Self::Address>
        + Serialize
        + DeserializeOwned;

    fn symbol() -> String;
}

pub mod bitcoin;
pub mod ethereum;
