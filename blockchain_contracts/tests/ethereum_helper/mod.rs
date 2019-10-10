pub mod tc_web3_client;
pub mod to_ethereum_address;
pub mod transaction;
pub mod wallet;

pub use self::{
    transaction::*,
    wallet::{InMemoryWallet, Wallet},
};

use lazy_static::lazy_static;
use rust_bitcoin::secp256k1::Secp256k1;

lazy_static! {
    pub static ref SECP: Secp256k1<rust_bitcoin::secp256k1::All> = Secp256k1::new();
}
