extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate common_types;
extern crate crypto;
#[macro_use]
extern crate lazy_static;
extern crate secp256k1;
extern crate secp256k1_support;
extern crate uuid;

use secp256k1::Secp256k1;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

mod key_store;

pub use key_store::*;
