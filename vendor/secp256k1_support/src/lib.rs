#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

#[macro_use]
extern crate lazy_static;

use hex;
use secp256k1;

pub use secp256k1::{constants::SECRET_KEY_SIZE, All, Secp256k1};
mod signature;
pub use crate::signature::*;
mod keypair;
pub use crate::keypair::*;
mod public_key;
pub use crate::public_key::*;

lazy_static! {
    pub static ref SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
}
