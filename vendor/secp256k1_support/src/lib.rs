extern crate secp256k1;
#[macro_use]
extern crate lazy_static;
extern crate rand;

pub use secp256k1::{constants::SECRET_KEY_SIZE, All, PublicKey, Secp256k1};
mod signature;
pub use signature::*;
mod keypair;
pub use keypair::*;

lazy_static! {
    static ref SECP: Secp256k1<secp256k1::All> = Secp256k1::new();
}
