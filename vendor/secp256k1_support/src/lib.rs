extern crate secp256k1;
#[macro_use]
extern crate lazy_static;
extern crate rand;

use secp256k1::Secp256k1;
pub use secp256k1::{PublicKey, SecretKey};
mod signature;
pub use signature::*;
mod keypair;
pub use keypair::*;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}
