use bitcoin::network::constants::Network;
use bitcoin::util::address::Address;
use bitcoin::util::bip32::ChildNumber;
use bitcoin::util::bip32::Error;
use bitcoin::util::bip32::ExtendedPrivKey;
use bitcoin::util::bip32::ExtendedPubKey;
use secp256k1::Secp256k1;

lazy_static! {
    static ref SECP: Secp256k1 = Secp256k1::new();
}

enum Type {
    AddressChain,
    // Derived from an Extended Public Key
    CustomKeys,
    // non-Bip32
    InternalChain, // Internal Chain as per bip32
}

use self::Type::*;

pub struct KeyStore {
    master_privkey: ExtendedPrivKey,
    htlc_root_privkey: ExtendedPrivKey,
    wallet_root_privkey: ExtendedPrivKey,
    last_wallet_index: u32,
    // Do we want to remember already generated addresses or regenerate them?
    // Memory vs CPU -> could be a switch/option
    // Common practice for wallets is to pre-generate some addresses, hence:
    // TODO: manage a key pool
    // - key ready for use (pool)
    // - key already used
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
            htlc_root_privkey,
            wallet_root_privkey,
            last_wallet_index: 0,
        }
    }

    pub fn get_new_wallet_privkey(&mut self) -> Result<ExtendedPrivKey, Error> {
        let res = self.wallet_root_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(self.last_wallet_index));
        if res.is_ok() {
            self.last_wallet_index += 1;
        }
        res
    }

    pub fn get_wallet_pubkey(&self, index: u32) -> Result<ExtendedPubKey, Error> {
        let priv_key = self.wallet_root_privkey
            .ckd_priv(&SECP, ChildNumber::Hardened(index))?;
        Ok(ExtendedPubKey::from_private(&SECP, &priv_key))
    }

    /*
    impl KeyStore {
        pub fn new_from_pub(xpubkey: ExtendedPubKey) -> KeyStore {
            KeyStore {
                store_type: AddressChain,
                master_pubkey: Some(xpubkey),
                master_privkey: None,
                last_chain_index: Some(0),
            }
        }

        pub fn new_pubkey(&mut self) -> Result<ExtendedPubKey, Error> {
            let res = self.xpubkey
                .ckd_pub(&self.secp, ChildNumber::Normal(self.last_index));
            if res.is_ok() {
                self.last_index += 1;
            }
            res
        }

        pub fn new_address(&mut self, network: Network) -> Result<Address, Error> {
            let pubkey = self.new_pubkey();
            match pubkey {
                Err(e) => return Err(e),
                Ok(pubkey) => {
                    // Using P2SH-WPKH (Legacy address wrapping SegWit)
                    // which is the most popular type of address at the moment
                    return Ok(Address::p2shwpkh(&pubkey.public_key, network));
                }
            }
        }
        */
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::key::PublicKey;
    use std::str::FromStr;

    #[test]
    fn get_wallet_priv_and_pub_keys() {
        let master_priv_key = ExtendedPrivKey::from_str(
            "xprv9s21ZrQH143K457pTbhs1LcmMnc4pCyqNTe9iEyoR8iTZeLtRzL6SpWCzK5iEP7fk72VhqkiNHuKQfqRVHTHBHQjxDDU7kTKHUuQCLNCbYi"
        ).unwrap();

        let mut keystore = KeyStore::new(master_priv_key);

        let wallet_privkey0 = keystore.get_new_wallet_privkey().unwrap();
        let wallet_privkey1 = keystore.get_new_wallet_privkey().unwrap();
        let wallet_privkey2 = keystore.get_new_wallet_privkey().unwrap();

        assert_eq!(wallet_privkey0.child_number, ChildNumber::Hardened(0));
        assert_eq!(wallet_privkey1.child_number, ChildNumber::Hardened(1));
        assert_eq!(wallet_privkey2.child_number, ChildNumber::Hardened(2));
        assert_ne!(wallet_privkey0, wallet_privkey1);
        assert_ne!(wallet_privkey1, wallet_privkey2);
        assert_ne!(wallet_privkey2, wallet_privkey0);

        let wallet_pubkey0 = keystore.get_wallet_pubkey(0).unwrap();
        let wallet_pubkey1 = keystore.get_wallet_pubkey(1).unwrap();
        let wallet_pubkey2 = keystore.get_wallet_pubkey(2).unwrap();

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
