//! Define domain specific terms using htlc_location module so that we can refer
//! to things in an ergonomic fashion e.g., `htlc_location::Bitcoin`.

pub use crate::ethereum::Address as Ethereum;
pub use bitcoin::OutPoint as Bitcoin;
