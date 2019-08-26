use crate::{fit_into_placeholder_slice::BitcoinTimestamp, FitIntoPlaceholderSlice, SecretHash};
use bitcoin::{
    network::constants::Network, util::bip143::SighashComponents, Address, OutPoint, Script,
    Transaction, TxIn, TxOut,
};
use secp256k1::{Message, PublicKey, SecretKey};
use std::borrow::Borrow;

// contract template RFC: https://github.com/comit-network/RFCs/blob/master/RFC-005-SWAP-Basic-Bitcoin.adoc#contract
pub const CONTRACT_TEMPLATE: [u8;118] = hex_literal::hex!("6382012088a82011111111111111111111111111111111111111111111111111111111111111118821033333333333333333333333333333333333333333333333333333333333333333ac6721022222222222222222222222222222222222222222222222222222222222222222ad0455a0fc01b168");

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
pub enum Error {
    FailedToSign(miniscript::Error),
    /// Subtracting the required fee from the input resulted in an underflow
    ///
    /// Encountering this error means there is not enough value in the HTLC to
    /// actually spend it. Redeeming or refunding such an HTLC is currently not
    /// supported and would require a child-pays-for-parent construct.
    FeeHigherThanInputValue,
}

#[derive(Debug)]
pub enum UnlockStrategy {
    Redeem { key: SecretKey, secret: [u8; 32] },
    Refund { key: SecretKey },
}

impl UnlockStrategy {
    fn expected_witness_stack_weight(&self) -> u64 {
        match self {
            UnlockStrategy::Redeem { .. } => REDEEM_TX_WITNESS_WEIGHT,
            UnlockStrategy::Refund { .. } => REFUND_TX_WITNESS_WEIGHT,
        }
    }
}

const REDEEM_TX_WITNESS_WEIGHT: u64 = 245;
const REFUND_TX_WITNESS_WEIGHT: u64 = 210;

impl BitcoinHtlc {
    pub fn new(
        expiry: u32,
        redeem_identity: secp256k1::PublicKey,
        refund_identity: secp256k1::PublicKey,
        secret_hash: [u8; 32],
    ) -> Self {
        let mut contract = CONTRACT_TEMPLATE.to_vec();
        SecretHash(secret_hash).fit_into_placeholder_slice(&mut contract[7..39]);
        redeem_identity.fit_into_placeholder_slice(&mut contract[41..74]);
        BitcoinTimestamp(expiry).fit_into_placeholder_slice(&mut contract[112..116]);
        refund_identity.fit_into_placeholder_slice(&mut contract[77..110]);

        let script = Script::from(contract);

        // TODO: this fails at the moment. find a way to re-construct the miniscript
        // from the compiled version so that we can rely on the hex but at the same time
        // use miniscript to create the spending input for us
        let miniscript = miniscript::Miniscript::parse(&script).expect("miniscript to parse");

        BitcoinHtlc {
            miniscript: miniscript::Descriptor::Wsh(miniscript),
            expiry,
        }
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
        fee_per_wu: u16,
        strategy: UnlockStrategy,
    ) -> Result<Transaction, Error> {
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
                value: 0, // overwritten once we estimated the weight
                script_pubkey: spend_to.script_pubkey(),
            }],
        };

        let base_tx_weight = spending_transaction.get_weight();
        let output_value = {
            let expected_weight = base_tx_weight + strategy.expected_witness_stack_weight();
            // This could potentially overflow but that would mean with have `weight` and
            // `fee` with absurd
            let fee = expected_weight * u64::from(fee_per_wu);

            input_value
                .checked_sub(fee)
                .ok_or(Error::FeeHigherThanInputValue)?
        };
        spending_transaction.output[0].value = output_value;

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
                    .satisfy(&mut htlc_tx_in, &statisfier, 0, 0)
                    .map_err(Error::FailedToSign)?;
            }
            Refund { key } => {
                let satisfier = RefundSatisfier {
                    secret_key: key,
                    signature: crate::SECP.sign(&message_to_sign, &key),
                };
                self.miniscript
                    .satisfy(&mut htlc_tx_in, &satisfier, 0, self.expiry)
                    .map_err(Error::FailedToSign)?;
            }
        }

        // Overwrite our input with the one containing the satisfied witness stack
        spending_transaction.input = vec![htlc_tx_in];

        let final_tx_weight = spending_transaction.get_weight();
        let final_tx_witness_stack_weight = final_tx_weight - base_tx_weight;
        let diff = diff(
            final_tx_witness_stack_weight,
            strategy.expected_witness_stack_weight(),
        );

        debug_assert!(
            diff < 10,
            "actual witness stack weight is {} and not {}, please update the const",
            final_tx_witness_stack_weight,
            strategy.expected_witness_stack_weight()
        );

        Ok(spending_transaction)
    }
}

fn diff(actual: u64, expected: u64) -> u64 {
    if actual > expected {
        actual - expected
    } else {
        expected - actual
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
        Some((
            to_bitcoin_public_key(&self.secret_key),
            (self.signature, bitcoin::SigHashType::All),
        ))
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
        Some((
            to_bitcoin_public_key(&self.secret_key),
            (self.signature, bitcoin::SigHashType::All),
        ))
    }

    fn lookup_sha256(&self, _: miniscript::bitcoin_hashes::sha256::Hash) -> Option<[u8; 32]> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_hashes::{sha256d, Hash};
    use secp256k1::rand::thread_rng;

    // just someone's public key
    fn an_identity() -> secp256k1::PublicKey {
        PublicKey::from_slice(&[
            3, 23, 183, 225, 206, 31, 159, 148, 195, 42, 67, 115, 146, 41, 248, 140, 11, 3, 51, 41,
            111, 180, 110, 143, 114, 134, 88, 73, 198, 174, 52, 184, 78,
        ])
        .unwrap()
    }

    #[test]
    fn constructor_does_not_panic() {
        BitcoinHtlc::new(100_000, an_identity(), an_identity(), [0u8; 32]);
    }

    #[quickcheck_macros::quickcheck]
    fn unlock_for_redeem_doesnt_panic(input_value: u64, fee_per_wu: u16) {
        let htlc = BitcoinHtlc::new(0, an_identity(), an_identity(), [0u8; 32]);
        let out_point = OutPoint {
            txid: sha256d::Hash::from_slice(&[0u8; 32]).unwrap(),
            vout: 0,
        };
        let (public_key, _) = crate::SECP.generate_keypair(&mut thread_rng());
        let address = "33iFwdLuRpW1uK1RTRqsoi8rR4NpDzk66k".parse().unwrap();

        let _ = htlc.unlock(
            out_point,
            input_value,
            address,
            fee_per_wu,
            UnlockStrategy::Redeem {
                key: public_key,
                secret: [0u8; 32],
            },
        );
    }

    #[quickcheck_macros::quickcheck]
    fn unlock_for_refund_doesnt_panic(input_value: u64, fee_per_wu: u16) {
        let htlc = BitcoinHtlc::new(100_000, an_identity(), an_identity(), [0u8; 32]);
        let out_point = OutPoint {
            txid: sha256d::Hash::from_slice(&[0u8; 32]).unwrap(),
            vout: 0,
        };
        let (public_key, _) = crate::SECP.generate_keypair(&mut thread_rng());
        let address = "33iFwdLuRpW1uK1RTRqsoi8rR4NpDzk66k".parse().unwrap();

        let _ = htlc.unlock(
            out_point,
            input_value,
            address,
            fee_per_wu,
            UnlockStrategy::Refund { key: public_key },
        );
    }
}
