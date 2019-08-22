use bitcoin::{
    network::constants::Network, util::bip143::SighashComponents, Address, OutPoint, Transaction,
    TxIn, TxOut,
};
use bitcoin_hashes::{hash160, hex::ToHex};
use secp256k1::{Message, PublicKey, SecretKey};
use std::{borrow::Borrow, str::FromStr};

// https://github.com/bitcoin/bips/blob/master/bip-0125.mediawiki
// Wallets that don't want to signal replaceability should use either a
// max sequence number (0xffffffff) or a sequence number of
//(0xffffffff-1) when then also want to use locktime;
pub const SEQUENCE_ALLOW_NTIMELOCK_NO_RBF: u32 = 0xFFFF_FFFE;

#[derive(Debug)]
pub struct BitcoinHtlc {
    miniscript: miniscript::Descriptor<bitcoin::PublicKey>,
    expiry: u32,
}

#[derive(Debug)]
pub enum UnlockStrategy {
    Redeem { key: SecretKey, secret: [u8; 32] },
    Refund { key: SecretKey },
}

impl BitcoinHtlc {
    pub fn new(
        expiry: u32,
        redeem_identity: hash160::Hash,
        refund_identity: hash160::Hash,
        secret_hash: [u8; 32],
    ) -> Self {
        let descriptor = format!(
            "wsh(c:or_i(and_v(v:sha256({secret_hash}),pk_h({redeem_identity})),and_v(v:older({expiry}),pk_h({refund_identity}))))",
            secret_hash = secret_hash.to_hex(),
            redeem_identity = redeem_identity,
            refund_identity = refund_identity,
            expiry = expiry,
        );

        let miniscript =
            miniscript::Descriptor::from_str(&descriptor).expect("descriptor to be valid");

        BitcoinHtlc { miniscript, expiry }
    }

    pub fn compute_address(&self, network: Network) -> Address {
        self.miniscript
            .address(network)
            .expect("script to be encodable to address")
    }

    pub fn unlock(
        self,
        htlc_location: OutPoint,
        input_value: u64,
        spend_to: bitcoin::Address,
        output_value: u64,
        strategy: UnlockStrategy,
    ) -> Result<Transaction, miniscript::Error> {
        use UnlockStrategy::*;

        let mut htlc_tx_in = TxIn {
            previous_output: htlc_location,
            script_sig: self.miniscript.unsigned_script_sig(),
            sequence: SEQUENCE_ALLOW_NTIMELOCK_NO_RBF,
            witness: vec![],
        };
        let lock_time = match strategy {
            Redeem { .. } => 0,
            Refund { .. } => self.expiry,
        };

        let mut spending_transaction = Transaction {
            version: 2,
            lock_time,
            input: vec![htlc_tx_in.clone()],
            output: vec![TxOut {
                value: output_value,
                script_pubkey: spend_to.script_pubkey(),
            }],
        };

        let sighash_components = SighashComponents::new(&spending_transaction);
        let hash_to_sign = sighash_components.sighash_all(
            &htlc_tx_in,
            &self.miniscript.witness_script(),
            input_value,
        );

        // `from` should be used instead of `from_slice` once `ThirtyTwoByteHash` is
        // implemented for Hashes See https://github.com/rust-bitcoin/rust-secp256k1/issues/106
        let message_to_sign = Message::from_slice(hash_to_sign.borrow())
            .expect("Should not fail because it is a hash");

        match strategy {
            Redeem { key, secret } => {
                let statisfier = RedeemStatisfier {
                    secret_key: key,
                    secret,
                    signature: crate::SECP.sign(&message_to_sign, &key),
                };
                self.miniscript
                    .satisfy(&mut htlc_tx_in, &statisfier, 0, 0)?;
            }
            Refund { key } => {
                let satisfier = RefundSatisfier {
                    secret_key: key,
                    signature: crate::SECP.sign(&message_to_sign, &key),
                };
                self.miniscript
                    .satisfy(&mut htlc_tx_in, &satisfier, 0, self.expiry)?;
            }
        }

        // Overwrite our input with the one containing the satisfied witness stack
        spending_transaction.input = vec![htlc_tx_in];

        Ok(spending_transaction)
    }
}

struct RedeemStatisfier {
    secret_key: SecretKey,
    signature: secp256k1::Signature,
    secret: [u8; 32],
}

struct RefundSatisfier {
    secret_key: SecretKey,
    signature: secp256k1::Signature,
}

fn to_bitcoin_public_key(secret_key: &SecretKey) -> bitcoin::PublicKey {
    let public_key = PublicKey::from_secret_key(&*crate::SECP, secret_key);

    bitcoin::PublicKey {
        compressed: true,
        key: public_key,
    }
}

impl miniscript::Satisfier<bitcoin::PublicKey> for RedeemStatisfier {
    fn lookup_pkh(
        &self,
        _: &<bitcoin::PublicKey as miniscript::MiniscriptKey>::Hash,
    ) -> Option<(
        bitcoin::PublicKey,
        (secp256k1::Signature, bitcoin::SigHashType),
    )> {
        return Some((
            to_bitcoin_public_key(&self.secret_key),
            (self.signature, bitcoin::SigHashType::All),
        ));
    }

    fn lookup_sha256(&self, _: miniscript::bitcoin_hashes::sha256::Hash) -> Option<[u8; 32]> {
        Some(self.secret)
    }
}

impl miniscript::Satisfier<bitcoin::PublicKey> for RefundSatisfier {
    fn lookup_pkh(
        &self,
        _: &<bitcoin::PublicKey as miniscript::MiniscriptKey>::Hash,
    ) -> Option<(
        bitcoin::PublicKey,
        (secp256k1::Signature, bitcoin::SigHashType),
    )> {
        return Some((
            to_bitcoin_public_key(&self.secret_key),
            (self.signature, bitcoin::SigHashType::All),
        ));
    }

    fn lookup_sha256(&self, _: miniscript::bitcoin_hashes::sha256::Hash) -> Option<[u8; 32]> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_hashes::{hash160, Hash};

    #[test]
    fn constructor_does_not_panic() {
        BitcoinHtlc::new(
            141241,
            hash160::Hash::from_slice(&[0u8; 20]).unwrap(),
            hash160::Hash::from_slice(&[0u8; 20]).unwrap(),
            [0u8; 32],
        );
    }
}
