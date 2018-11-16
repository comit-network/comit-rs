use bitcoin_support::{BitcoinQuantity, FindOutput, OutPoint, Transaction};
use swap_protocols::{
    ledger::Bitcoin,
    rfc003::{
        state_machine::HtlcParams,
        validation::{Error, IsContainedInTransaction},
    },
};

impl IsContainedInTransaction<Bitcoin> for BitcoinQuantity {
    fn is_contained_in_transaction(
        htlc_params: &HtlcParams<Bitcoin, BitcoinQuantity>,
        transaction: Transaction,
    ) -> Result<OutPoint, Error<BitcoinQuantity>> {
        let address = htlc_params.compute_address();

        let (vout, txout) = transaction
            .find_output(&address)
            .ok_or(Error::WrongTransaction)?;

        let location = OutPoint {
            txid: transaction.txid(),
            vout: vout as u32,
        };

        let actual_value = BitcoinQuantity::from_satoshi(txout.value);
        let required_value = htlc_params.asset;

        debug!("Value of HTLC at {:?} is {}", location, actual_value);

        let has_enough_money = actual_value >= required_value;

        trace!(
            "{} >= {} -> {}",
            actual_value,
            required_value,
            has_enough_money
        );
        if has_enough_money {
            Ok(location)
        } else {
            Err(Error::UnexpectedAsset {
                found: actual_value,
                expected: required_value,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate bitcoin_support;

    use super::{Error as ValidationError, *};
    use bitcoin_rpc_client::rpc::{
        ScriptPubKey, ScriptType, SerializedRawTransaction, TransactionOutput,
        VerboseRawTransaction,
    };
    use bitcoin_support::{BitcoinQuantity, Blocks, Sha256dHash, Transaction};
    use hex::FromHex;
    use spectral::prelude::*;
    use swap_protocols::rfc003::{state_machine::*, Secret};

    fn gen_htlc_params(bitcoin_amount: f64) -> HtlcParams<Bitcoin, BitcoinQuantity> {
        HtlcParams {
            asset: BitcoinQuantity::from_bitcoin(bitcoin_amount),
            ledger: Bitcoin::regtest(),
            success_identity: bitcoin_support::PubkeyHash::from_hex(
                "d38e554430c4035f2877a579a07a99886153f071",
            )
            .unwrap(),
            refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "d38e554430c4035f2877a579a07a99886153f072",
            )
            .unwrap(),
            lock_duration: Blocks::from(144),
            secret_hash: Secret::from(*b"hello world, you are beautiful!!").into(),
        }
    }

    #[test]
    fn transaction_contains_output_with_sufficient_money() {
        let bitcoin_amount = 1.0;
        let htlc_params = gen_htlc_params(bitcoin_amount);
        let script = htlc_params.compute_address().script_pubkey();

        let script_pub_key = ScriptPubKey {
            asm: String::from(""),
            hex: script.clone(),
            req_sigs: None,
            script_type: ScriptType::NullData,
            addresses: None,
        };

        let transaction_output = TransactionOutput {
            value: htlc_params.asset.bitcoin(),
            n: 1,
            script_pub_key,
        };

        let transaction = VerboseRawTransaction {
            txid: Sha256dHash::from_data(b"a"),
            hash: String::from(""),
            size: 0,
            vsize: 0,
            version: 1,
            locktime: 42,
            vin: Vec::new(),
            vout: vec![transaction_output],
            hex: SerializedRawTransaction(String::from("")),
            blockhash: Sha256dHash::from_data(b"blockhash"),
            confirmations: 0,
            time: 0,
            blocktime: 0,
        };

        let bitcoin_transaction: Transaction = transaction.into();

        let result =
            BitcoinQuantity::is_contained_in_transaction(&htlc_params, bitcoin_transaction.clone());

        let txid = bitcoin_transaction.txid();
        let expected_outpoint = OutPoint { txid, vout: 0 };

        assert_that(&result).is_ok_containing(expected_outpoint)
    }

    #[test]
    fn transaction_does_not_contain_output() {
        let bitcoin_amount = 1.0;

        let transaction = VerboseRawTransaction {
            txid: Sha256dHash::from_data(b"refunded"),
            hash: String::from(""),
            size: 0,
            vsize: 0,
            version: 1,
            locktime: 42,
            vin: Vec::new(),
            vout: Vec::new(),
            hex: SerializedRawTransaction(String::from("")),
            blockhash: Sha256dHash::from_data(b"blockhash"),
            confirmations: 0,
            time: 0,
            blocktime: 0,
        };

        let result = BitcoinQuantity::is_contained_in_transaction(
            &gen_htlc_params(bitcoin_amount),
            transaction.into(),
        );

        assert_that(&result).is_err_containing(ValidationError::WrongTransaction)
    }

    #[test]
    fn transaction_does_not_contain_enough_money() {
        let bitcoin_amount = 1.0;
        let htlc_params = gen_htlc_params(bitcoin_amount);

        let script = htlc_params.compute_address().script_pubkey();
        let script_pub_key = ScriptPubKey {
            asm: String::from(""),
            hex: script.clone(),
            req_sigs: None,
            script_type: ScriptType::NullData,
            addresses: None,
        };

        let provided_bitcoin_amount = 0.5;

        let transaction_output = TransactionOutput {
            value: provided_bitcoin_amount,
            n: 1,
            script_pub_key,
        };

        let transaction = VerboseRawTransaction {
            txid: Sha256dHash::from_data(b"a"),
            hash: String::from(""),
            size: 0,
            vsize: 0,
            version: 1,
            locktime: 42,
            vin: Vec::new(),
            vout: vec![transaction_output],
            hex: SerializedRawTransaction(String::from("")),
            blockhash: Sha256dHash::from_data(b"blockhash"),
            confirmations: 0,
            time: 0,
            blocktime: 0,
        };

        let result = BitcoinQuantity::is_contained_in_transaction(&htlc_params, transaction.into());

        let expected_error = ValidationError::UnexpectedAsset {
            found: BitcoinQuantity::from_bitcoin(provided_bitcoin_amount),
            expected: BitcoinQuantity::from_bitcoin(bitcoin_amount),
        };

        assert_that(&result).is_err_containing(expected_error)
    }
}
