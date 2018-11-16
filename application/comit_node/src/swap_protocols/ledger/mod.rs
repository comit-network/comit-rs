use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

mod bitcoin;
mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};
use bam_api::header::{FromBamHeader, ToBamHeader};
use http_api::ledger::{FromHttpLedger, ToHttpLedger};

pub trait Ledger:
    Clone
    + Debug
    + Send
    + Sync
    + 'static
    + Default
    + PartialEq
    + FromHttpLedger
    + ToHttpLedger
    + FromBamHeader
    + ToBamHeader
{
    type Quantity: Debug + Copy + DeserializeOwned + Serialize + Send + Sync + 'static;
    type TxId: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + PartialEq + 'static;
    type Pubkey: Clone + Debug + Send + Sync + 'static;
    type Address: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Identity: Clone
        + Debug
        + Send
        + Sync
        + PartialEq
        + 'static
        + From<Self::Address>
        + Serialize
        + DeserializeOwned;
    type Transaction: Debug
        + Clone
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + PartialEq
        + 'static;

    fn address_for_identity(&self, identity: Self::Identity) -> Self::Address;
}
