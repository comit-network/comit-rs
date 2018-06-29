use bitcoin::util::bip32::ChildNumber;
use bitcoin::util::bip32::Error;
use bitcoin::util::bip32::ExtendedPrivKey;
use bitcoin::util::bip32::ExtendedPubKey;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use secp256k1::Secp256k1;
use secp256k1::key::PublicKey;
use secp256k1::key::SecretKey;
// Not sure if I want this library to be aware of the Uuid concept
use std::collections::HashMap;
use uuid::Uuid;

// TODO: Move in a constant mod
/// The size (in bytes) of a secret key
pub const SECRET_KEY_SIZE: usize = 32;

pub const SHA256_DIGEST_LENGTH: usize = 32;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

#[derive(Clone)]
pub struct IdBasedPrivKey {
    pub secret_key: SecretKey,
    pub source_id: Uuid,
}

#[derive(Clone)]
pub struct IdBasedPubKey {
    pub public_key: PublicKey,
    pub source_id: Uuid,
}

#[derive(Clone)]
pub struct KeyPair(SecretKey, PublicKey);

#[derive(Clone)]
pub struct IdBasedKeyPair {
    pub uid: Uuid,
    keys: KeyPair,
}

impl IdBasedKeyPair {
    fn get_secret_key(&self) -> &SecretKey {
        &self.keys.0
    }

    fn get_public_key(&self) -> &PublicKey {
        &self.keys.1
    }
}

pub struct KeyStore {
    master_privkey: ExtendedPrivKey,
    id_based_root_privkey: ExtendedPrivKey,
    internal_root_privkey: ExtendedPrivKey,
    last_wallet_index: u32,
    // Do we want to remember already generated addresses or regenerate them?
    // Memory vs CPU -> could be a switch/option
    // Common practice for wallets is to pre-generate some addresses, hence:
    // TODO: manage a key pool
    // - key ready for use (pool)
    // - key already used
    id_based_keys: HashMap<Uuid, IdBasedKeyPair>, // Better generate Public Key from SecretKey on the fly or storing them?
}

impl KeyStore {
    pub fn new(master_privkey: ExtendedPrivKey) -> KeyStore {
        let temp_hardened_privkey = master_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(0))
            .expect("Could not derive m/'0");
        // m/'0/'2
        let htlc_root_privkey = temp_hardened_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(2))
            .expect("Could not derive m/'0/'2");
        // m/'0/'0
        let wallet_root_privkey = temp_hardened_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(0))
            .expect("Could not derive m/'0/'0");

        KeyStore {
            master_privkey,
            id_based_root_privkey: htlc_root_privkey,
            internal_root_privkey: wallet_root_privkey,
            last_wallet_index: 0,
            id_based_keys: HashMap::new(),
        }
    }

    pub fn get_new_internal_privkey(&mut self) -> Result<ExtendedPrivKey, Error> {
        let res = self.internal_root_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(self.last_wallet_index));
        if res.is_ok() {
            self.last_wallet_index += 1;
        }
        res
    }

    pub fn get_internal_pubkey(&self, index: u32) -> Result<ExtendedPubKey, Error> {
        let priv_key = self.internal_root_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(index))?;
        Ok(ExtendedPubKey::from_private(&SECP, &priv_key))
    }

    fn get_id_based_keypair(&mut self, id: &Uuid) -> Result<IdBasedKeyPair, Error> {
        let id_based_root_privkey = &self.id_based_root_privkey;
        // I don't like this bool but I don't like any other way I tried either
        let mut needs_insert = false;
        let id_based_keypair = match self.id_based_keys.get(id) {
            None => {
                needs_insert = true;
                let key_pair = Self::new_id_based_keys(id_based_root_privkey, id);
                key_pair
            }
            Some(key_pair) => {
                return Ok(key_pair.clone());
            }
        }?;

        if needs_insert {
            self.id_based_keys
                .insert(id.clone(), id_based_keypair.clone());
        }
        Ok(id_based_keypair)
    }

    fn new_id_based_keys(
        id_based_root_privkey: &ExtendedPrivKey,
        uid: &Uuid,
    ) -> Result<IdBasedKeyPair, Error> {
        // SecretKey = SHA256(id_based_root_privkey + id)
        let root_key = id_based_root_privkey.secret_key;
        let root_key: &[u8] = &root_key[..];

        let id = uid.as_bytes();
        let input = ([root_key, id]).concat();

        let mut sha = Sha256::new();
        sha.input(&input[..]);

        let mut result: [u8; SHA256_DIGEST_LENGTH] = [0; SHA256_DIGEST_LENGTH];
        sha.result(&mut result);

        let secret_key = SecretKey::from_slice(&SECP, &result)?;
        let public_key = PublicKey::from_secret_key(&SECP, &secret_key)?;
        Ok(IdBasedKeyPair {
            uid: uid.clone(),
            keys: KeyPair(secret_key, public_key),
        })
    }

    pub fn get_id_based_privkey(&mut self, id: &Uuid) -> Result<IdBasedPrivKey, Error> {
        let key_pair = self.get_id_based_keypair(id)?;
        Ok(IdBasedPrivKey {
            secret_key: key_pair.get_secret_key().clone(),
            source_id: id.clone(),
        })
    }

    pub fn get_id_based_pubkey(&mut self, id: &Uuid) -> Result<IdBasedPubKey, Error> {
        let key_pair = self.get_id_based_keypair(id)?;
        Ok(IdBasedPubKey {
            public_key: key_pair.get_public_key().clone(),
            source_id: id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn get_internal_priv_and_pub_keys() {
        let master_priv_key = ExtendedPrivKey::from_str(
            "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        ).unwrap();

        let mut keystore = KeyStore::new(master_priv_key);

        let internal_privkey0 = keystore.get_new_internal_privkey().unwrap();
        let internal_privkey1 = keystore.get_new_internal_privkey().unwrap();
        let internal_privkey2 = keystore.get_new_internal_privkey().unwrap();

        assert_eq!(internal_privkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(internal_privkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(internal_privkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(internal_privkey0, internal_privkey1);
        assert_ne!(internal_privkey1, internal_privkey2);
        assert_ne!(internal_privkey2, internal_privkey0);

        let internal_pubkey0 = keystore.get_internal_pubkey(0).unwrap();
        let internal_pubkey1 = keystore.get_internal_pubkey(1).unwrap();
        let internal_pubkey2 = keystore.get_internal_pubkey(2).unwrap();

        // All key pairs have been verified as valid using `ku -P` the bip32 python reference
        println!("{}", internal_privkey0.to_string());
        println!("{}", internal_privkey1.to_string());
        println!("{}", internal_privkey2.to_string());
        println!("{}", internal_pubkey0.to_string());
        println!("{}", internal_pubkey1.to_string());
        println!("{}", internal_pubkey2.to_string());

        assert_eq!(internal_pubkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(internal_pubkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(internal_pubkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(internal_pubkey0, internal_pubkey1);
        assert_ne!(internal_pubkey1, internal_pubkey2);
        assert_ne!(internal_pubkey2, internal_pubkey0);

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
    fn generate_diff_id_based_keys() {
        let master_priv_key = ExtendedPrivKey::from_str(
            "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        ).unwrap();

        let mut keystore = KeyStore::new(master_priv_key);

        let uid0 = Uuid::new_v4();
        let uid1 = Uuid::new_v4();
        let uid2 = Uuid::new_v4();

        let privkey0 = keystore.get_id_based_privkey(&uid0).unwrap();
        let privkey1 = keystore.get_id_based_privkey(&uid1).unwrap();
        let privkey2 = keystore.get_id_based_privkey(&uid2).unwrap();
        let pubkey0 = keystore.get_id_based_pubkey(&uid0).unwrap();
        let pubkey1 = keystore.get_id_based_pubkey(&uid1).unwrap();
        let pubkey2 = keystore.get_id_based_pubkey(&uid2).unwrap();

        let pubkey_from_priv0 = PublicKey::from_secret_key(&SECP, &privkey0.secret_key).unwrap();
        let pubkey_from_priv1 = PublicKey::from_secret_key(&SECP, &privkey1.secret_key).unwrap();
        let pubkey_from_priv2 = PublicKey::from_secret_key(&SECP, &privkey2.secret_key).unwrap();

        assert_eq!(pubkey_from_priv0, pubkey0.public_key);
        assert_eq!(pubkey_from_priv1, pubkey1.public_key);
        assert_eq!(pubkey_from_priv2, pubkey2.public_key);
    }
}
