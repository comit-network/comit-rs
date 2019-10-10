#![warn(unused_extern_crates, rust_2018_idioms)]
#![forbid(unsafe_code)]
#![allow(clippy::print_stdout)]

use bitcoin::{Network, PrivateKey};
use bitcoin_support::PubkeyHash;
use ethereum_support::Address;
use secp256k1_omni_context::{
    secp256k1::{self, PublicKey, Secp256k1},
    Builder,
};
use std::env;

fn main() {
    let secp: Secp256k1<secp256k1::All> = Secp256k1::new();
    let secret_key = match env::args().nth(1) {
        Some(existing_key) => Builder::new(secp.clone())
            .secret_key_hex(existing_key.as_ref())
            .unwrap()
            .build()
            .unwrap(),
        None => {
            let mut rng = secp256k1_omni_context::secp256k1::rand::OsRng::new().unwrap();
            Builder::new(secp).rng(&mut rng).build().unwrap()
        }
    };

    let (secret_key, public_key) = secret_key.keys();
    let mainnet_private_key = PrivateKey {
        compressed: true,
        network: Network::Bitcoin,
        key: secret_key,
    };
    let testnet_private_key = PrivateKey {
        compressed: true,
        network: Network::Testnet,
        key: secret_key,
    };

    println!("private_key: {}", hex::encode(&secret_key[..]));
    println!(
        "WIF_mainnet_private_key: {}",
        mainnet_private_key.to_string()
    );
    println!(
        "WIF_testnet_private_key: {}",
        testnet_private_key.to_string()
    );
    println!("public_key: {}", hex::encode(&public_key.serialize()[..]));
    println!(
        "public_key_uncompressed: {}",
        hex::encode(&public_key.serialize_uncompressed()[..])
    );
    let eth_address = to_ethereum_address(&public_key);
    println!("eth_address: {:?}", eth_address);
    {
        let btc_address_mainnet = bitcoin::Address::p2wpkh(
            &bitcoin::PublicKey {
                compressed: true,
                key: public_key,
            },
            Network::Bitcoin,
        );
        println!("btc_address_p2wpkh_mainnet: {:?}", btc_address_mainnet);
    }

    {
        let btc_address_testnet = bitcoin::Address::p2wpkh(
            &bitcoin::PublicKey {
                compressed: true,
                key: public_key,
            },
            Network::Testnet,
        );
        println!("btc_address_p2wpkh_testnet: {:?}", btc_address_testnet);
    }
    {
        let btc_address_regtest = bitcoin::Address::p2wpkh(
            &bitcoin::PublicKey {
                compressed: true,
                key: public_key,
            },
            Network::Regtest,
        );
        println!("btc_address_p2wpkh_regtest: {:?}", btc_address_regtest);
    }
    println!("pubkey_hash: {:x}", PubkeyHash::from(public_key));
}

fn to_ethereum_address(key: &PublicKey) -> Address {
    let serialized_public_key = key.serialize_uncompressed();
    // Remove the silly openssl 0x04 byte from the front of the
    // serialized public key. This is a bitcoin thing that
    // ethereum doesn't want. Eth pubkey should be 32 + 32 = 64 bytes.
    let actual_public_key = &serialized_public_key[1..];
    let hash = tiny_keccak::keccak256(actual_public_key);
    // Ethereum address is the last twenty bytes of the keccak256 hash
    let ethereum_address_bytes = &hash[12..];
    let mut address = Address::default();
    address.assign_from_slice(ethereum_address_bytes);
    address
}
