extern crate bitcoin_support;
extern crate secp256k1_support;

use bitcoin_support::{ChainCode, ChildNumber, ExtendedPrivKey, Fingerprint};
use secp256k1_support::{SecretKey, SECP};

pub fn extended_privkey_from_array(
    secret_key_data: &[u8; 32],
    network: bitcoin_support::Network,
) -> ExtendedPrivKey {
    // safe unwrap as it only fails if secret_key_data.len != 32
    let secret_key = SecretKey::from_slice(&SECP, &secret_key_data[..]).unwrap();
    extended_privkey_from_secret_key(&secret_key, network)
}

pub fn extended_privkey_from_secret_key(
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
