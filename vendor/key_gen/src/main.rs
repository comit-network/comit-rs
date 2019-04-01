use bitcoin_support::{IntoP2wpkhAddress, Network, PrivateKey, PubkeyHash};
use ethereum_support::ToEthereumAddress;
use rand::OsRng;
use secp256k1_support::KeyPair;
use std::env;

fn main() {
    let keypair = match env::args().skip(1).next() {
        Some(existing_key) => KeyPair::from_secret_key_hex(existing_key.as_ref()).unwrap(),
        None => {
            let mut rng = OsRng::new().unwrap();
            KeyPair::new(&mut rng)
        }
    };

    let secret_key = keypair.secret_key();
    let public_key = keypair.public_key();
    let mainnet_private_key =
        PrivateKey::from_secret_key(secret_key, true, Network::Mainnet.into());
    let testnet_private_key =
        PrivateKey::from_secret_key(secret_key, true, Network::Testnet.into());

    println!("private_key: {}", hex::encode(&secret_key[..]));
    println!(
        "WIF_mainnet_private_key: {}",
        mainnet_private_key.to_string()
    );
    println!(
        "WIF_testnet_private_key: {}",
        testnet_private_key.to_string()
    );
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
        let btc_address_mainnet = public_key.into_p2wpkh_address(Network::Mainnet);
        println!("btc_address_p2wpkh_mainnet: {:?}", btc_address_mainnet);
    }

    {
        let btc_address_testnet = public_key.into_p2wpkh_address(Network::Testnet);
        println!("btc_address_p2wpkh_testnet: {:?}", btc_address_testnet);
    }
    {
        let btc_address_regtest = public_key.into_p2wpkh_address(Network::Regtest);
        println!("btc_address_p2wpkh_regtest: {:?}", btc_address_regtest);
    }
    println!("pubkey_hash: {:x}", PubkeyHash::from(public_key));
}
