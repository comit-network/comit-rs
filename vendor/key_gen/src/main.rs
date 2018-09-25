extern crate bitcoin_support;
extern crate ethereum_support;
extern crate hex;
extern crate rand;
extern crate secp256k1_support;

use bitcoin_support::{
    ChainCode, ChildNumber, ExtendedPrivKey, ExtendedPubKey, Fingerprint, Network, PrivateKey,
    PubkeyHash, ToP2wpkhAddress,
};
use ethereum_support::ToEthereumAddress;
use rand::OsRng;
use secp256k1_support::{KeyPair, SecretKey, SECP};

fn main() {
    let mut rng = OsRng::new().unwrap();
    let keypair = KeyPair::new(&mut rng);
    let secret_key = keypair.secret_key();
    let public_key = keypair.public_key();
    let private_key = PrivateKey::from_secret_key(*secret_key, true, Network::Bitcoin);

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

    {
        let extended_privkey = extended_privkey_from_secret_key(secret_key, Network::Bitcoin);
        let extended_pubkey = ExtendedPubKey::from_private(&SECP, &extended_privkey);
        println!("btc_extended_privkey_mainnet: {}", extended_privkey);
        println!("btc_extended_pubkey_mainnet: {}", extended_pubkey);
    }
    {
        let extended_privkey = extended_privkey_from_secret_key(secret_key, Network::Testnet);
        let extended_pubkey = ExtendedPubKey::from_private(&SECP, &extended_privkey);
        println!("btc_extended_privkey_testnet: {}", extended_privkey);
        println!("btc_extended_pubkey_testnet: {}", extended_pubkey);
    }
    {
        let extended_privkey = extended_privkey_from_secret_key(secret_key, Network::Regtest);
        let extended_pubkey = ExtendedPubKey::from_private(&SECP, &extended_privkey);
        println!("btc_extended_privkey_regtest: {}", extended_privkey);
        println!("btc_extended_pubkey_regtest: {}", extended_pubkey);
    }
}

fn extended_privkey_from_secret_key(
    secret_key: &SecretKey,
    network: bitcoin_support::Network,
) -> ExtendedPrivKey {
    let chain_code = ChainCode::from(&[1u8; 32][..]);
    ExtendedPrivKey {
        network,
        depth: 0,
        parent_fingerprint: Fingerprint::default(),
        child_number: ChildNumber::from(0),
        secret_key: secret_key.clone(),
        chain_code,
    }
}
