#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod bitcoin;
pub mod ethereum;
mod fit_into_placeholder_slice;

pub use self::fit_into_placeholder_slice::{
    FitIntoPlaceholderSlice, SecretHash, Timestamp, TokenQuantity,
};
