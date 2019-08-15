use crate::{
    fit_into_placeholder_slice::{BitcoinTimestamp, FitIntoPlaceholderSlice},
    SecretHash,
};
use bitcoin::{network::constants::Network, Address, Script};
use bitcoin_hashes::hash160;
use bitcoin_witness::{UnlockParameters, Witness, SEQUENCE_ALLOW_NTIMELOCK_NO_RBF};
use hex_literal::hex;
use secp256k1::{PublicKey, SecretKey};

// contract template RFC: https://github.com/comit-network/RFCs/blob/master/RFC-005-SWAP-Basic-Bitcoin.adoc#contract
pub const CONTRACT_TEMPLATE: [u8;97] = hex!("6382012088a82010000000000000000000000000000000000000000000000000000000000000018876a9143000000000000000000000000000000000000003670420000002b17576a91440000000000000000000000000000000000000046888ac");

#[derive(Debug)]
pub enum UnlockingError {
    WrongSecret {
        got: SecretHash,
        expected: SecretHash,
    },
    WrongPubkeyHash {
        got: [u8; 20],
        expected: [u8; 20],
    },
}

#[derive(Debug)]
pub struct BitcoinHtlc {
    script: Vec<u8>,
    expiry: u32,
}

impl BitcoinHtlc {
    pub fn new(
        expiry: u32,
        refund_identity: hash160::Hash,
        redeem_identity: hash160::Hash,
        secret_hash: [u8; 32],
    ) -> Self {
        let mut contract = CONTRACT_TEMPLATE.to_vec();
        SecretHash(secret_hash).fit_into_placeholder_slice(&mut contract[7..39]);
        redeem_identity.fit_into_placeholder_slice(&mut contract[43..63]);
        BitcoinTimestamp(expiry).fit_into_placeholder_slice(&mut contract[65..69]);
        refund_identity.fit_into_placeholder_slice(&mut contract[74..94]);

        BitcoinHtlc {
            script: contract,
            expiry,
        }
    }

    pub fn compute_address(&self, network: Network) -> Address {
        Address::p2wsh(&Script::from(self.script.clone()), network)
    }

    pub fn unlock_with_secret(self, secret_key: SecretKey, secret: [u8; 32]) -> UnlockParameters {
        let public_key = PublicKey::from_secret_key(&*crate::SECP, &secret_key);
        UnlockParameters {
            witness: vec![
                Witness::Signature(secret_key),
                Witness::PublicKey(public_key),
                Witness::Data(secret.to_vec()),
                Witness::Bool(true),
                Witness::PrevScript,
            ],
            sequence: SEQUENCE_ALLOW_NTIMELOCK_NO_RBF,
            locktime: 0,
            prev_script: self.into_script(),
        }
    }

    pub fn unlock_after_timeout(self, secret_key: SecretKey) -> UnlockParameters {
        let public_key = PublicKey::from_secret_key(&*crate::SECP, &secret_key);
        UnlockParameters {
            witness: vec![
                Witness::Signature(secret_key),
                Witness::PublicKey(public_key),
                Witness::Bool(false),
                Witness::PrevScript,
            ],
            sequence: SEQUENCE_ALLOW_NTIMELOCK_NO_RBF,
            locktime: self.expiry,
            prev_script: self.into_script(),
        }
    }

    fn into_script(self) -> Script {
        Script::from(self.script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_hashes::hash160;
    use regex::bytes::Regex;

    const SECRET_HASH: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        0, 1,
    ];

    const SECRET_HASH_REGEX: &'static str = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x00\x01";

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = BitcoinHtlc::new(
            3000000,
            hash160::Hash::default(),
            hash160::Hash::default(),
            SECRET_HASH,
        );

        assert_eq!(
            htlc.script.len(),
            CONTRACT_TEMPLATE.len(),
            "HTLC is the same length as template"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_contain_given_data() {
        let htlc = BitcoinHtlc::new(
            2000000000,
            hash160::Hash::default(),
            hash160::Hash::default(),
            SECRET_HASH,
        );

        let _re_match = Regex::new(SECRET_HASH_REGEX)
            .expect("Could not create regex")
            .find(&htlc.script)
            .expect("Could not find secret hash in hex code");
    }
}
