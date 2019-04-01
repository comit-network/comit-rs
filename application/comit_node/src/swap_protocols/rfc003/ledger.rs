use crate::swap_protocols;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

pub trait Ledger: swap_protocols::Ledger {
	type HtlcLocation: PartialEq + Debug + Clone + DeserializeOwned + Serialize + Send + Sync;
}
