use crypto::{digest::Digest, sha2::Sha256};
use hex::FromHexError;
use hex_literal::hex;
use std::{str::FromStr, thread::sleep, time::Duration};
use web3::types::Address as EthereumAddress;

mod erc20_harness;
mod ether_harness;
mod timestamp;

pub use self::{
    erc20_harness::{erc20_harness, Erc20HarnessParams},
    ether_harness::{ether_harness, EtherHarnessParams},
    timestamp::Timestamp,
};
use crate::ethereum_helper::{to_ethereum_address::ToEthereumAddress, SECP};
use rust_bitcoin::secp256k1::{PublicKey, SecretKey};

pub fn new_account(secret_key: &str) -> (SecretKey, EthereumAddress) {
    let secret_key = SecretKey::from_str(secret_key).unwrap();
    let public_key = PublicKey::from_secret_key(&*SECP, &secret_key);
    (secret_key, public_key.to_ethereum_address())
}

pub const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";
pub const SECRET_HASH: [u8; 32] =
    hex!("68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec");

#[derive(Debug)]
pub struct CustomSizeSecret(pub Vec<u8>);

impl CustomSizeSecret {
    pub fn hash(&self) -> [u8; 32] {
        let mut sha = Sha256::new();
        sha.input(&self.0[..]);

        let mut result = [0; 32];
        sha.result(&mut result);

        result
    }
}

impl FromStr for CustomSizeSecret {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let secret = s.as_bytes().to_vec();
        Ok(CustomSizeSecret(secret))
    }
}

fn diff(first: Timestamp, second: Timestamp) -> u32 {
    u32::from(first).saturating_sub(u32::from(second))
}

pub fn sleep_until(timestamp: Timestamp) {
    let duration = diff(timestamp, Timestamp::now());
    let buffer = 2;

    sleep(Duration::from_secs((duration + buffer).into()));
}
