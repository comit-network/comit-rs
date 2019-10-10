#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub use crate::secret_key::*;
pub use secp256k1;

mod secret_key;
