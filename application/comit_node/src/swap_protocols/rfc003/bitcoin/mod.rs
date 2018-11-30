use bitcoin_support::{Address, BitcoinQuantity, Blocks, OutPoint};
use key_store::KeyStore;
use secp256k1_support::KeyPair;
use swap_protocols::{
    ledger::Bitcoin,
    rfc003::{
        ledger::{HttpRefundIdentity, HttpSuccessIdentity},
        secret::{Secret, SecretHash},
        state_machine::HtlcParams,
        IntoHtlcIdentity, Ledger, RedeemTransaction,
    },
};
use swaps::common::SwapId;

mod actions;
mod htlc;
mod queries;
mod validation;

pub use self::{
    actions::*,
    htlc::{Htlc, UnlockingError},
    queries::*,
};

impl Ledger for Bitcoin {
    type LockDuration = Blocks;
    type HtlcLocation = OutPoint;
    type HtlcIdentity = KeyPair;
    type HttpIdentity = ();

    fn extract_secret(
        transaction: &RedeemTransaction<Self>,
        secret_hash: &SecretHash,
    ) -> Option<Secret> {
        transaction.as_ref().input.iter().find_map(|txin| {
            txin.witness
                .iter()
                .find_map(|script_item| match Secret::from_vec(&script_item) {
                    Ok(secret) => match secret.hash() == *secret_hash {
                        true => Some(secret),
                        false => None,
                    },
                    Err(_) => None,
                })
        })
    }
}

impl IntoHtlcIdentity<Bitcoin> for HttpSuccessIdentity<()> {
    fn into_htlc_identity(self, swap_id: SwapId, key_store: &KeyStore) -> KeyPair {
        key_store.get_transient_keypair(&swap_id.into(), b"SUCCESS")
    }
}

impl IntoHtlcIdentity<Bitcoin> for HttpRefundIdentity<()> {
    fn into_htlc_identity(self, swap_id: SwapId, key_store: &KeyStore) -> KeyPair {
        key_store.get_transient_keypair(&swap_id.into(), b"REFUND")
    }
}

impl From<HtlcParams<Bitcoin, BitcoinQuantity>> for Htlc {
    fn from(htlc_params: HtlcParams<Bitcoin, BitcoinQuantity>) -> Self {
        Htlc::new(
            htlc_params.success_identity,
            htlc_params.refund_identity,
            htlc_params.secret_hash,
            htlc_params.lock_duration.into(),
        )
    }
}

impl HtlcParams<Bitcoin, BitcoinQuantity> {
    pub fn compute_address(&self) -> Address {
        Htlc::from(self.clone()).compute_address(self.ledger.network)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::{serialize::deserialize, OutPoint, Script, Transaction, TxIn};
    use hex;
    use spectral::prelude::*;
    use std::str::FromStr;

    fn setup(secret: &Secret) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: Script::new(),
                sequence: 0,
                witness: vec![
                    vec![],                       // Signature
                    vec![],                       // Public key
                    secret.raw_secret().to_vec(), // Secret
                    vec![1u8],                    // Bool to enter redeem branch
                    vec![],                       // Previous Script
                ],
            }],
            output: vec![],
        }
    }

    #[test]
    fn extract_correct_secret() {
        let secret = Secret::from(*b"This is our favourite passphrase");
        let transaction = setup(&secret);

        assert_that!(Bitcoin::extract_secret(
            &RedeemTransaction(transaction),
            &secret.hash()
        ))
        .is_some()
        .is_equal_to(&secret);
    }

    #[test]
    fn extract_incorrect_secret() {
        let secret = Secret::from(*b"This is our favourite passphrase");
        let transaction = setup(&secret);

        let secret_hash = SecretHash::from_str(
            "bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf\
             bfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbf",
        )
        .unwrap();
        assert_that!(Bitcoin::extract_secret(
            &RedeemTransaction(transaction),
            &secret_hash
        ))
        .is_none();
    }

    #[test]
    fn extract_correct_secret_from_mainnet_transaction() {
        let hex_tx = hex::decode("0200000000010124e06fe5594b941d06c7385dc7307ec694a41f7d307423121855ee17e47e06ad0100000000ffffffff0137aa0b000000000017a914050377baa6e8c5a07aed125d0ef262c6d5b67a038705483045022100d780139514f39ed943179e4638a519101bae875ec1220b226002bcbcb147830b0220273d1efb1514a77ee3dd4adee0e896b7e76be56c6d8e73470ae9bd91c91d700c01210344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f20ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e01015b63a82091d6a24697ed31932537ae598d3de3131e1fcd0641b9ac4be7afcb376386d71e8876a9149f4a0cf348b478336cb1d87ea4c8313a7ca3de1967029000b27576a91465252e57f727a27f32c77098e14d88d8dbec01816888ac00000000").unwrap();
        let transaction: Transaction = deserialize(&hex_tx).unwrap();
        let hex_secret =
            hex::decode("ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e")
                .unwrap();
        let secret = Secret::from_vec(&hex_secret).unwrap();

        assert_that!(Bitcoin::extract_secret(
            &RedeemTransaction(transaction),
            &secret.hash()
        ))
        .is_some()
        .is_equal_to(&secret);
    }
}
