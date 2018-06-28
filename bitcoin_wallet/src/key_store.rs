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
    id_based_privkeys: HashMap<Uuid, IdBasedPrivKey>,
    id_based_pubkeys: HashMap<Uuid, IdBasedPubKey>,
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
            id_based_privkeys: HashMap::new(),
            id_based_pubkeys: HashMap::new(),
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

    pub fn get_id_based_privkey(&mut self, id: &Uuid) -> Result<IdBasedPrivKey, Error> {
        let id_based_privkey = match self.id_based_privkeys.get(id) {
            None => {
                let privkey = { self.new_id_based_privkey(id) };
                privkey
            }
            Some(privkey) => {
                return Ok(privkey.clone());
            }
        }?;

        self.id_based_privkeys
            .insert(id.clone(), id_based_privkey.clone());
        Ok(id_based_privkey)
    }

    fn new_id_based_privkey(&self, uid: &Uuid) -> Result<IdBasedPrivKey, Error> {
        // SecretKey = SHA256(id_based_root_privkey + id)
        let root_key = self.id_based_root_privkey.secret_key;
        let root_key: &[u8] = &root_key[..];

        let id = uid.as_bytes();
        let input = ([root_key, id]).concat();

        let mut sha = Sha256::new();
        sha.input(&input[..]);

        let mut result: [u8; SHA256_DIGEST_LENGTH] = [0; SHA256_DIGEST_LENGTH];
        sha.result(&mut result);

        let secret_key = SecretKey::from_slice(&SECP, &result)?;

        Ok(IdBasedPrivKey {
            secret_key,
            source_id: uid.clone(),
        })
    }

    pub fn get_id_based_pubkey(&mut self, id: &Uuid) -> Result<IdBasedPubKey, Error> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn get_wallet_priv_and_pub_keys() {
        let master_priv_key = ExtendedPrivKey::from_str(
            "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        ).unwrap();

        let mut keystore = KeyStore::new(master_priv_key);

        let wallet_privkey0 = keystore.get_new_internal_privkey().unwrap();
        let wallet_privkey1 = keystore.get_new_internal_privkey().unwrap();
        let wallet_privkey2 = keystore.get_new_internal_privkey().unwrap();

        assert_eq!(wallet_privkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(wallet_privkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(wallet_privkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(wallet_privkey0, wallet_privkey1);
        assert_ne!(wallet_privkey1, wallet_privkey2);
        assert_ne!(wallet_privkey2, wallet_privkey0);

        let wallet_pubkey0 = keystore.get_internal_pubkey(0).unwrap();
        let wallet_pubkey1 = keystore.get_internal_pubkey(1).unwrap();
        let wallet_pubkey2 = keystore.get_internal_pubkey(2).unwrap();

        assert_eq!(wallet_pubkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(wallet_pubkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(wallet_pubkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(wallet_pubkey0, wallet_pubkey1);
        assert_ne!(wallet_pubkey1, wallet_pubkey2);
        assert_ne!(wallet_pubkey2, wallet_pubkey0);

        let pubkey_from_priv0 =
            PublicKey::from_secret_key(&SECP, &wallet_privkey0.secret_key).unwrap();
        let pubkey_from_priv1 =
            PublicKey::from_secret_key(&SECP, &wallet_privkey1.secret_key).unwrap();
        let pubkey_from_priv2 =
            PublicKey::from_secret_key(&SECP, &wallet_privkey2.secret_key).unwrap();
        let pub_key_from_ext0 = wallet_pubkey0.public_key;
        let pub_key_from_ext1 = wallet_pubkey1.public_key;
        let pub_key_from_ext2 = wallet_pubkey2.public_key;
        assert_eq!(pubkey_from_priv0, pub_key_from_ext0);
        assert_eq!(pubkey_from_priv1, pub_key_from_ext1);
        assert_eq!(pubkey_from_priv2, pub_key_from_ext2);
    }
    /*#[test]
    fn given_bip32_vector1_m0h_pubkey_return_correct_m0h1_pubkey() {
        // See https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki Test vector 1
        // Chain m/0H
        let pub_key = ExtendedPubKey::from_str("xpub68Gmy5EdvgibQVfPdqkBBCHxA5htiqg55crXYuXoQRKfDBFA1WEjWgP6LHhwBZeNK1VTsfTFUHCdrfp1bgwQ9xv5ski8PX9rL2dZXvgGDnw").unwrap();
        // Chain m/0H/1
        let expected_pubkey = ExtendedPubKey::from_str("xpub6ASuArnXKPbfEwhqN6e3mwBcDTgzisQN1wXN9BJcM47sSikHjJf3UFHKkNAWbWMiGj7Wf5uMash7SyYq527Hqck2AxYysAA7xmALppuCkwQ").unwrap();

        let mut add_gen = KeyStore::new(pub_key);
        // Chain m/0H/0 (discard)
        let _ = add_gen.new_pubkey();

        assert_eq!(add_gen.new_pubkey(), Ok(expected_pubkey));
    }*/
}
