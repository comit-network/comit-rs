extern crate secp256k1;
#[macro_use]
extern crate lazy_static;
extern crate hex;
extern crate rand;
extern crate serde;

pub use secp256k1::{constants::SECRET_KEY_SIZE, All, Secp256k1};
mod signature;
pub use signature::*;
mod keypair;
pub use keypair::*;
mod public_key;
pub use public_key::*;

lazy_static! {
    static ref SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
}
