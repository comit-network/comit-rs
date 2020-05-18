//! Define domain specific terms using transaction module so that we can refer
//! to things in an ergonomic fashion e.g., `transaction::Ethereum`.

pub use crate::ethereum::Transaction as Ethereum;
pub use bitcoin::Transaction as Bitcoin;
