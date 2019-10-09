#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use lazy_static::lazy_static;
use rust_bitcoin::secp256k1::Secp256k1;

pub mod bitcoin;
pub mod ethereum;
mod fit_into_placeholder_slice;

pub use self::fit_into_placeholder_slice::{
    EthereumTimestamp, FitIntoPlaceholderSlice, SecretHash, TokenQuantity,
};

// TODO: Remove this, only binary should instantiate a secp context
lazy_static! {
    pub static ref SECP: Secp256k1<rust_bitcoin::secp256k1::All> = Secp256k1::new();
}
