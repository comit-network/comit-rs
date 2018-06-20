extern crate bitcoin;
extern crate bitcoin_wallet;
extern crate ethereum_wallet;
extern crate hex;
extern crate rand;
extern crate secp256k1;

use bitcoin::network::constants::Network;
use bitcoin_wallet::{PrivateKey, ToP2wpkhAddress};
use ethereum_wallet::ToEthereumAddress;
use rand::OsRng;
use secp256k1::Secp256k1;
use secp256k1::key::{PublicKey, SecretKey};

fn main() {
    let secp = Secp256k1::new();
    let mut rng = OsRng::new().unwrap();
    let secret_key = SecretKey::new(&secp, &mut rng);
    let public_key = PublicKey::from_secret_key(&secp, &secret_key).unwrap();
    let private_key = PrivateKey::from_secret_key(secret_key, false, Network::Bitcoin);

    println!("private_key: {}", hex::encode(&secret_key[..]));
    println!("btc_base58_private_key: {}", private_key.to_string());
    println!("public_key: {}", hex::encode(&public_key.serialize()[..]));
    println!(
        "public_key_uncompressed: {}",
        hex::encode(&public_key.serialize_uncompressed()[..])
    );
    let eth_address = public_key.to_ethereum_address();
    println!("eth_address: {:?}", eth_address);
    let btc_address_mainnet = public_key.to_p2wpkh_address(Network::Bitcoin);
    println!("btc_address_p2wpkh_mainnet: {:?}", btc_address_mainnet);
}
