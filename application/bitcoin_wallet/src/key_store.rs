use super::SECP;
use bitcoin::util::bip32::{self, ChildNumber, ExtendedPrivKey, ExtendedPubKey};
use crypto::{digest::Digest, sha2::Sha256};
use secp256k1::constants::SECRET_KEY_SIZE;
use secp256k1_support::KeyPair;
use std::{
    collections::HashMap,
    ops::DerefMut,
    sync::{Mutex, MutexGuard, PoisonError},
};
use uuid::Uuid;

#[derive(Debug)]
pub enum Error {
    Bip32Error(bip32::Error),
    IndexLockError,
}

impl From<bip32::Error> for Error {
    fn from(e: bip32::Error) -> Self {
        Error::Bip32Error(e)
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, u32>>> for Error {
    fn from(_e: PoisonError<MutexGuard<'a, u32>>) -> Self {
        Error::IndexLockError
    }
}

pub struct KeyStore {
    master_privkey: ExtendedPrivKey,
    transient_root_privkey: ExtendedPrivKey,
    internal_root_privkey: ExtendedPrivKey,
    last_internal_index: Mutex<u32>,
    // Do we want to remember already generated addresses or regenerate them?
    // Memory vs CPU -> could be a switch/option
    // Common practice for wallets is to pre-generate some addresses, hence:
    // TODO: manage a key pool
    // - key ready for use (pool)
    // - key already used
    transient_keys: HashMap<Uuid, KeyPair>,
}

impl KeyStore {
    pub fn new(master_privkey: ExtendedPrivKey) -> Result<KeyStore, Error> {
        // As per bip32 and bitcoind reference implementation
        //
        // We use the following child keys:
        // m/0'/0' for bip32 internal chain (ie, where the BTC is sent after redeem, bitcoind-like)
        // m/0'/2' for HTLC (ie, locking the money in HTLC). 2 is an arbitrary value I chose
        // (1' being reserved for the bip32 external chain)
        // At this stage we expect an extended master private key in the configuration (m)
        // Then we just assume that we use account 0' (like bitcoind), hence we derive m/0'
        // and create our child keys from there.

        let account_0_privkey = master_privkey.ckd_priv(&SECP, ChildNumber::Hardened(0))?;

        let internal_root_privkey = account_0_privkey.ckd_priv(&SECP, ChildNumber::Hardened(0))?;
        let transient_root_privkey = account_0_privkey.ckd_priv(&SECP, ChildNumber::Hardened(2))?;

        Ok(KeyStore {
            master_privkey,
            transient_root_privkey: transient_root_privkey,
            internal_root_privkey: internal_root_privkey,
            last_internal_index: Mutex::new(0),
            transient_keys: HashMap::new(),
        })
    }

    pub fn get_new_internal_privkey(&self) -> Result<ExtendedPrivKey, Error> {
        let mut lock = self.last_internal_index.lock()?;
        let index = lock.deref_mut();

        let res = self
            .internal_root_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(*index))?;

        // If we reach here, res is Ok
        *index += 1;
        Ok(res)
    }

    pub fn get_internal_pubkey(&self, index: u32) -> Result<ExtendedPubKey, Error> {
        let priv_key = self
            .internal_root_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(index))?;
        Ok(ExtendedPubKey::from_private(&SECP, &priv_key))
    }

    pub fn get_transient_keypair(&mut self, id: &Uuid) -> KeyPair {
        if let Some(keypair) = self.transient_keys.get(id) {
            return keypair.clone();
        }

        let transient_keypair = Self::new_transient_keypair(&self.transient_root_privkey, id);
        self.transient_keys
            .insert(id.clone(), transient_keypair.clone());
        transient_keypair
    }

    fn new_secret_from_concat(data1: &[u8], data2: &[u8], secret: &mut [u8]) {
        let mut sha = Sha256::new();
        sha.input(data1);
        sha.input(data2);
        sha.result(secret);
    }

    fn new_transient_keypair(transient_root_privkey: &ExtendedPrivKey, uid: &Uuid) -> KeyPair {
        // SecretKey = SHA256(transient_root_privkey + id)
        let mut result: [u8; SECRET_KEY_SIZE] = [0; SECRET_KEY_SIZE];

        Self::new_secret_from_concat(
            &transient_root_privkey.secret_key[..],
            &uid.as_bytes()[..],
            &mut result,
        );
        // This returns a result as it can fail if the slice is empty which is very unlikely hence the expect.
        KeyPair::from_secret_key_slice(&result).expect("Should never fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1_support::PublicKey;
    use std::str::FromStr;

    fn setup_keystore() -> KeyStore {
        let master_priv_key = ExtendedPrivKey::from_str(
        "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        ).unwrap();

        KeyStore::new(master_priv_key).unwrap()
    }

    #[test]
    fn internal_priv_and_pub_keys_sequential_generation() {
        let keystore = setup_keystore();

        let internal_privkey0 = keystore.get_new_internal_privkey().unwrap();
        let internal_privkey1 = keystore.get_new_internal_privkey().unwrap();
        let internal_privkey2 = keystore.get_new_internal_privkey().unwrap();

        assert_eq!(internal_privkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(internal_privkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(internal_privkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(internal_privkey0, internal_privkey1);
        assert_ne!(internal_privkey1, internal_privkey2);
        assert_ne!(internal_privkey2, internal_privkey0);
    }

    #[test]
    fn internal_key_generation_child_numbers_are_correct() {
        let keystore = setup_keystore();

        let internal_pubkey0 = keystore.get_internal_pubkey(0).unwrap();
        let internal_pubkey1 = keystore.get_internal_pubkey(1).unwrap();
        let internal_pubkey2 = keystore.get_internal_pubkey(2).unwrap();

        assert_eq!(internal_pubkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(internal_pubkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(internal_pubkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(internal_pubkey0, internal_pubkey1);
        assert_ne!(internal_pubkey1, internal_pubkey2);
        assert_ne!(internal_pubkey2, internal_pubkey0);
    }

    #[test]
    fn internal_key_generation_pub_keys_match() {
        let keystore = setup_keystore();

        let internal_privkey0 = keystore.get_new_internal_privkey().unwrap();
        let internal_privkey1 = keystore.get_new_internal_privkey().unwrap();
        let internal_privkey2 = keystore.get_new_internal_privkey().unwrap();
        let internal_pubkey0 = keystore.get_internal_pubkey(0).unwrap();
        let internal_pubkey1 = keystore.get_internal_pubkey(1).unwrap();
        let internal_pubkey2 = keystore.get_internal_pubkey(2).unwrap();

        let pubkey_from_priv0 =
            PublicKey::from_secret_key(&SECP, &internal_privkey0.secret_key).unwrap();
        let pubkey_from_priv1 =
            PublicKey::from_secret_key(&SECP, &internal_privkey1.secret_key).unwrap();
        let pubkey_from_priv2 =
            PublicKey::from_secret_key(&SECP, &internal_privkey2.secret_key).unwrap();
        let pub_key_from_ext0 = internal_pubkey0.public_key;
        let pub_key_from_ext1 = internal_pubkey1.public_key;
        let pub_key_from_ext2 = internal_pubkey2.public_key;
        assert_eq!(pubkey_from_priv0, pub_key_from_ext0);
        assert_eq!(pubkey_from_priv1, pub_key_from_ext1);
        assert_eq!(pubkey_from_priv2, pub_key_from_ext2);
    }

    #[test]
    fn generate_diff_transient_keys() {
        let master_priv_key = ExtendedPrivKey::from_str(
            "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        ).unwrap();

        let mut keystore = KeyStore::new(master_priv_key).unwrap();

        let uid0 = Uuid::new_v4();
        let uid1 = Uuid::new_v4();

        let transient_keypair0 = keystore.get_transient_keypair(&uid0);
        let transient_keypair1 = keystore.get_transient_keypair(&uid1);

        assert_ne!(transient_keypair0, transient_keypair1);
    }
}
