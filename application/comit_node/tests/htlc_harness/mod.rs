use comit_node::swap_protocols::rfc003::SecretHash;
use crypto::{digest::Digest, sha2::Sha256};
use ethereum_support::{web3::types::Address as EthereumAddress, ToEthereumAddress};
use hex::FromHexError;
use secp256k1_support::KeyPair;
use std::str::FromStr;

mod erc20_harness;
mod ether_harness;

pub use self::{
    erc20_harness::{erc20_harness, Erc20HarnessParams},
    ether_harness::{ether_harness, EtherHarnessParams},
};

pub fn new_account(secret_key: &str) -> (KeyPair, EthereumAddress) {
    let keypair = KeyPair::from_secret_key_hex(secret_key).unwrap();
    let address = keypair.public_key().to_ethereum_address();

    (keypair, address)
}

pub const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";

#[derive(Debug)]
pub struct CustomSizeSecret(pub Vec<u8>);

impl CustomSizeSecret {
    pub fn hash(&self) -> SecretHash {
        let mut sha = Sha256::new();
        sha.input(&self.0[..]);

        let mut result: [u8; SecretHash::LENGTH] = [0; SecretHash::LENGTH];
        sha.result(&mut result);
        SecretHash::from(result)
    }
}

impl FromStr for CustomSizeSecret {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let secret = s.as_bytes().to_vec();
        Ok(CustomSizeSecret(secret))
    }
}
