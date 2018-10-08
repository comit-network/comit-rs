use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, str::FromStr};
use swap_protocols;

pub trait Ledger:
    Clone + Debug + Send + Sync + 'static + Default + Into<swap_protocols::wire_types::Ledger>
{
    type Quantity: Debug + Copy + DeserializeOwned + Serialize + Send + Sync + 'static;
    type LockDuration: Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static;
    type HtlcId: Clone + DeserializeOwned + Serialize + Send + Sync;
    type TxId: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + FromStr + 'static;
    type Pubkey: Clone + Debug + Send + Sync + 'static;
    type Address: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Identity: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + From<Self::Address>
        + Serialize
        + DeserializeOwned;

    type QueryForLedgerQueryService: Clone + Debug + Send + Sync;

    fn symbol() -> String;
    fn address_for_identity(&self, Self::Identity) -> Self::Address;
}

pub mod bitcoin;
pub mod ethereum;
