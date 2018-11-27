use ethereum_support::{web3::types::Address, ToEthereumAddress};
use secp256k1_support::KeyPair;
use std::time::Duration;

mod erc20_harness;
mod ether_harness;

pub use self::{
    erc20_harness::{erc20_harness, Erc20HarnessParams},
    ether_harness::{ether_harness, EtherHarnessParams},
};

pub fn new_account(secret_key: &str) -> (KeyPair, Address) {
    let keypair = KeyPair::from_secret_key_hex(secret_key).unwrap();
    let address = keypair.public_key().to_ethereum_address();

    (keypair, address)
}

pub const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";
pub const HTLC_TIMEOUT: Duration = Duration::from_secs(5);
