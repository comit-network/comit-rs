extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate common_types;
extern crate crypto;
#[macro_use]
extern crate lazy_static;
extern crate secp256k1;
extern crate uuid;

pub use bitcoin::blockdata::script::Script;
pub use bitcoin::util::address::Address;
use secp256k1::Secp256k1;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

mod bitcoin_wallet;
mod key;
mod key_store;
mod weight;

pub use bitcoin_wallet::*;
pub use key::*;
pub use key_store::*;
pub use weight::*;
