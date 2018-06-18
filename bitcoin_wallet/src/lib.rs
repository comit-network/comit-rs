extern crate bitcoin;
extern crate secp256k1;
#[macro_use]
extern crate lazy_static;
extern crate bitcoin_rpc;
extern crate common_types;

pub use bitcoin::blockdata::script::Script;
pub use bitcoin::util::address::Address;
use secp256k1::Secp256k1;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

mod bitcoin_wallet;
mod key;
pub use bitcoin_wallet::*;
pub use key::*;
