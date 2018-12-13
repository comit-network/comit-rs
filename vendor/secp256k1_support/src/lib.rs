#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate lazy_static;

pub use crate::{keypair::*, public_key::*, signature::*};
pub use secp256k1::{constants::SECRET_KEY_SIZE, All, Secp256k1};

mod keypair;
mod public_key;
mod signature;

lazy_static! {
    pub static ref SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
}
