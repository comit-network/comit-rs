use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use swap_protocols;

mod bitcoin;
mod ethereum;

pub use self::{bitcoin::Bitcoin, ethereum::Ethereum};

pub trait Ledger:
    Clone
    + Debug
    + Send
    + Sync
    + 'static
    + Default
    + PartialEq
    + Into<swap_protocols::wire_types::Ledger>
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

    fn symbol() -> String;
    fn address_for_identity(&self, identity: Self::Identity) -> Self::Address;
}
