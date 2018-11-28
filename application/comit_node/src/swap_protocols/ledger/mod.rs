use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

mod bitcoin;
mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};
use bam_api::header::{FromBamHeader, ToBamHeader};
use http_api::ledger::{FromHttpLedger, ToHttpLedger};
use std::hash::Hash;

pub trait Ledger:
    Clone
    + Debug
    + Send
    + Sync
    + 'static
    + Default
    + PartialEq
    + Eq
    + Hash
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
        + Eq
        + Hash
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
    type HttpIdentity: DeserializeOwned;

    fn address_for_identity(&self, identity: Self::Identity) -> Self::Address;
}
