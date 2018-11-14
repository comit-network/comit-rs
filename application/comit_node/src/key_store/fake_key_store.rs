use bitcoin_support::{ChainCode, ChildNumber, ExtendedPrivKey, Fingerprint, Network};
use key_store::KeyStore;
use secp256k1_support::{SecretKey, SECP};

#[derive(Debug)]
pub struct FakeKeyStoreFactory(KeyStore);

impl FakeKeyStoreFactory {
    pub fn create() -> KeyStore {
        let secret_key = SecretKey::from_slice(
            &SECP,
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8,
                9, 0, 1, 2,
            ][..],
        )
        .unwrap();
        let extended_privkey = ExtendedPrivKey {
            network: Network::Regtest,
            depth: 0,
            parent_fingerprint: Fingerprint::default(),
            child_number: ChildNumber::from(0),
            secret_key,
            chain_code: ChainCode::from(&[1u8; 32][..]),
        };

        KeyStore::new(extended_privkey).expect("Could not HD derive keys from the private key")
    }
}
