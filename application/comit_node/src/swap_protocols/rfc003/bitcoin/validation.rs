use crate::swap_protocols::{
    ledger::Bitcoin,
    rfc003::{
        find_htlc_location::{compare_assets, Error, FindHtlcLocation},
        state_machine::HtlcParams,
    },
};
use bitcoin_support::{BitcoinQuantity, FindOutput, OutPoint, Transaction};

impl FindHtlcLocation<Bitcoin, BitcoinQuantity> for Transaction {
    fn find_htlc_location(
        &self,
        htlc_params: &HtlcParams<Bitcoin, BitcoinQuantity>,
    ) -> Result<OutPoint, Error<BitcoinQuantity>> {
        let address = htlc_params.compute_address();

        let (vout, txout) = self.find_output(&address).ok_or(Error::WrongTransaction)?;

        let location = OutPoint {
            txid: self.txid(),
            vout,
        };

        let actual_value = BitcoinQuantity::from_satoshi(txout.value);
        let required_value = htlc_params.asset;

        compare_assets(location, actual_value, required_value)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error as ValidationError, *};
    use crate::swap_protocols::rfc003::{state_machine::*, Secret, Timestamp};
    use bitcoin_support::{self, BitcoinQuantity, Transaction, TxOut};
    use hex::FromHex;
    use spectral::prelude::*;

    fn gen_htlc_params(bitcoin_amount: BitcoinQuantity) -> HtlcParams<Bitcoin, BitcoinQuantity> {
        HtlcParams {
            asset: bitcoin_amount,
            ledger: Bitcoin::default(),
            redeem_identity: bitcoin_support::PubkeyHash::from_hex(
                "d38e554430c4035f2877a579a07a99886153f071",
            )
            .unwrap(),
            refund_identity: bitcoin_support::PubkeyHash::from_hex(
                "d38e554430c4035f2877a579a07a99886153f072",
            )
            .unwrap(),
            expiry: Timestamp(123456789),
            secret_hash: Secret::from(*b"hello world, you are beautiful!!").into(),
        }
    }

    #[test]
    fn transaction_contains_output_with_sufficient_money() {
        let bitcoin_amount = BitcoinQuantity::from_bitcoin(1.0);
        let htlc_params = gen_htlc_params(bitcoin_amount);
        let script_pubkey = htlc_params.compute_address().script_pubkey();

        let transaction_output = TxOut {
            value: htlc_params.asset.satoshi(),
            script_pubkey,
        };

        let transaction = Transaction {
            version: 1,
            lock_time: 42,
            input: vec![],
            output: vec![transaction_output],
        };

        let bitcoin_transaction: Transaction = transaction.into();

        let result = bitcoin_transaction.clone().find_htlc_location(&htlc_params);

        let txid = bitcoin_transaction.txid();
        let expected_outpoint = OutPoint { txid, vout: 0 };

        assert_that(&result).is_ok_containing(expected_outpoint)
    }

    #[test]
    fn transaction_does_not_contain_output() {
        let bitcoin_amount = BitcoinQuantity::from_bitcoin(1.0);
        let transaction = Transaction {
            version: 1,
            lock_time: 42,
            input: vec![],
            output: vec![],
        };

        let result = transaction.find_htlc_location(&gen_htlc_params(bitcoin_amount));

        assert_that(&result).is_err_containing(ValidationError::WrongTransaction)
    }

    #[test]
    fn transaction_does_not_contain_enough_money() {
        let bitcoin_amount = BitcoinQuantity::from_bitcoin(1.0);
        let htlc_params = gen_htlc_params(bitcoin_amount);

        let script_pubkey = htlc_params.compute_address().script_pubkey();

        let provided_bitcoin_amount = BitcoinQuantity::from_bitcoin(0.5);

        let transaction_output = TxOut {
            value: provided_bitcoin_amount.satoshi(),
            script_pubkey,
        };

        let transaction = Transaction {
            version: 1,
            lock_time: 42,
            input: vec![],
            output: vec![transaction_output],
        };

        let result = transaction.find_htlc_location(&htlc_params);

        let expected_error = ValidationError::UnexpectedAsset {
            found: provided_bitcoin_amount,
            expected: bitcoin_amount,
        };

        assert_that(&result).is_err_containing(expected_error)
    }
}
