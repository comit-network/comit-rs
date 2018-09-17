extern crate bitcoin;
extern crate bitcoin_support;
extern crate ethereum_support;
extern crate hex;
extern crate rand;
extern crate secp256k1_support;

use bitcoin::network::constants::Network;
use bitcoin_support::{PrivateKey, PubkeyHash, ToP2wpkhAddress};
use ethereum_support::ToEthereumAddress;
use rand::OsRng;
use secp256k1_support::KeyPair;

fn main() {
    let mut rng = OsRng::new().unwrap();
    let keypair = KeyPair::new(&mut rng);
    let secret_key = keypair.secret_key();
    let public_key = keypair.public_key();
    let private_key = PrivateKey::from_secret_key(secret_key.clone(), true, Network::Bitcoin);

    println!("private_key: {}", hex::encode(&secret_key[..]));
    println!("btc_base58_private_key: {}", private_key.to_string());
    println!(
        "public_key: {}",
        hex::encode(&public_key.inner().serialize()[..])
    );
    println!(
        "public_key_uncompressed: {}",
        hex::encode(&public_key.inner().serialize_uncompressed()[..])
    );
    let eth_address = public_key.to_ethereum_address();
    println!("eth_address: {:?}", eth_address);
    {
        let btc_address_mainnet = public_key.to_p2wpkh_address(Network::Bitcoin);
        println!("btc_address_p2wpkh_mainnet: {:?}", btc_address_mainnet);
    }

    {
        let btc_address_testnet = public_key.to_p2wpkh_address(Network::Testnet);
        println!("btc_address_p2wpkh_testnet: {:?}", btc_address_testnet);
    }
    {
        let btc_address_regtest = public_key.to_p2wpkh_address(Network::Regtest);
        println!("btc_address_p2wpkh_regtest: {:?}", btc_address_regtest);
    }
    println!("pubkey_hash: {:?}", PubkeyHash::from(public_key.clone()));
}
