//! Define domain specific terms using identity module so that we can refer to
//! things in an ergonomic fashion e.g., `identity::Bitcoin`.

pub use crate::{bitcoin::PublicKey as Bitcoin, ethereum::Address as Ethereum};
