use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

pub trait Ledger: Clone + Debug + Send + Sync + 'static {
    type Quantity: Debug + Copy + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Address: Debug + Clone + DeserializeOwned + Serialize + Send + Sync + 'static;
    type Time;
    type HtlcId: Clone + DeserializeOwned + Serialize;

    fn symbol() -> String;
}

pub mod bitcoin;
pub mod ethereum;
