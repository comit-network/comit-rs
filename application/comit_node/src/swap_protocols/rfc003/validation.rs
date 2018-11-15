use bitcoin_support::{BitcoinQuantity, FindOutput, OutPoint};
use ethereum_support::{self, CalculateContractAddress, EtherQuantity};
use swap_protocols::{
    asset::Asset,
    ledger::{Bitcoin, Ethereum},
    rfc003::{
        self, bitcoin::bitcoin_htlc_address, ethereum::ethereum_htlc, state_machine::OngoingSwap,
        IntoSecretHash,
    },
    Ledger,
};

#[derive(Debug, PartialEq)]
pub enum Error<A: Asset> {
    UnexpectedAsset { found: A, expected: A },
    WrongTransaction,
}

pub trait IsContainedInSourceLedgerTransaction<SL, TL, TA, S>: Send + Sync
where
    SL: rfc003::Ledger,
    TL: rfc003::Ledger,
    Self: Asset,
    TA: Asset,
    S: IntoSecretHash,
{
    fn is_contained_in_source_ledger_transaction(
        swap: OngoingSwap<SL, TL, Self, TA, S>,
        transaction: SL::Transaction,
    ) -> Result<SL::HtlcLocation, Error<Self>>;
}

pub trait IsContainedInTargetLedgerTransaction<SL, TL, SA, S>: Send + Sync
where
    SL: rfc003::Ledger,
    TL: rfc003::Ledger,
    SA: Asset,
    Self: Asset,
    S: IntoSecretHash,
{
    fn is_contained_in_target_ledger_transaction(
        swap: OngoingSwap<SL, TL, SA, Self, S>,
        tx: TL::Transaction,
    ) -> Result<TL::HtlcLocation, Error<Self>>;
}

impl<TL, TA, S> IsContainedInSourceLedgerTransaction<Bitcoin, TL, TA, S> for BitcoinQuantity
where
    TL: rfc003::Ledger,
    TA: Asset,
    S: IntoSecretHash,
{
    fn is_contained_in_source_ledger_transaction(
        swap: OngoingSwap<Bitcoin, TL, BitcoinQuantity, TA, S>,
        transaction: <Bitcoin as Ledger>::Transaction,
    ) -> Result<OutPoint, Error<BitcoinQuantity>> {
        let address = bitcoin_htlc_address(&swap);

        let (vout, txout) = transaction
            .find_output(&address)
            .ok_or(Error::WrongTransaction)?;

        let location = OutPoint {
            txid: transaction.txid(),
            vout: vout as u32,
        };

        let actual_value = BitcoinQuantity::from_satoshi(txout.value);
        let required_value = swap.source_asset;

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

impl<SL, SA, S> IsContainedInTargetLedgerTransaction<SL, Ethereum, SA, S> for EtherQuantity
where
    SL: rfc003::Ledger,
    SA: Asset,
    S: IntoSecretHash,
{
    fn is_contained_in_target_ledger_transaction(
        swap: OngoingSwap<SL, Ethereum, SA, EtherQuantity, S>,
        tx: ethereum_support::Transaction,
    ) -> Result<ethereum_support::Address, Error<EtherQuantity>> {
        if tx.to != None {
            return Err(Error::WrongTransaction);
        }

        if tx.input != ethereum_htlc(&swap).compile_to_hex().into() {
            return Err(Error::WrongTransaction);
        }

        if tx.value < swap.target_asset.wei() {
            return Err(Error::UnexpectedAsset {
                found: EtherQuantity::from_wei(tx.value),
                expected: swap.target_asset,
            });
        }

        let from_address: ethereum_support::Address = tx.from;

        Ok(from_address.calculate_contract_address(&tx.nonce))
    }
}

#[cfg(test)]
mod tests {
    extern crate bitcoin_support;
    extern crate ethereum_support;
    extern crate hex;
    extern crate secp256k1_support;

    use super::{Error as ValidationError, *};
    use bitcoin_rpc_client::rpc::{
        ScriptPubKey, ScriptType, SerializedRawTransaction, TransactionOutput,
        VerboseRawTransaction,
    };
    use bitcoin_support::{BitcoinQuantity, Blocks, Sha256dHash, Transaction};
    use ethereum_support::{
        web3::types::{Bytes, H256, U256},
        EtherQuantity,
    };
    use hex::FromHex;
    use spectral::prelude::*;
    use std::str::FromStr;
    use swap_protocols::{
        ledger::Ethereum,
        rfc003::{
            ethereum::{ethereum_htlc, Seconds},
            state_machine::*,
            AcceptResponse, Secret,
        },
    };

    fn gen_start_state(
        bitcoin_amount: f64,
        ether_amount: U256,
    ) -> Start<Bitcoin, Ethereum, BitcoinQuantity, EtherQuantity, Secret> {
        Start {
            source_ledger_refund_identity: secp256k1_support::KeyPair::from_secret_key_slice(
                &hex::decode("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725")
                    .unwrap(),
            )
            .unwrap(),
            target_ledger_success_identity: ethereum_support::Address::from_str(
                "8457037fcd80a8650c4692d7fcfc1d0a96b92867",
            )
            .unwrap(),
            source_ledger: Bitcoin::regtest(),
            target_ledger: Ethereum::default(),
            source_asset: BitcoinQuantity::from_bitcoin(bitcoin_amount),
            target_asset: EtherQuantity::from_wei(ether_amount),
            source_ledger_lock_duration: Blocks::from(144),
            secret: Secret::from(*b"hello world, you are beautiful!!"),
        }
    }

    fn gen_response() -> AcceptResponse<Bitcoin, Ethereum> {
        AcceptResponse {
            target_ledger_refund_identity: ethereum_support::Address::from_str(
                "71b9f69dcabb340a3fe229c3f94f1662ad85e5e8",
            )
            .unwrap(),
            source_ledger_success_identity: bitcoin_support::PubkeyHash::from_hex(
                "d38e554430c4035f2877a579a07a99886153f071",
            )
            .unwrap(),
            target_ledger_lock_duration: Seconds(42),
        }
    }

    #[test]
    fn bitcoin_transaction_contains_output_with_sufficient_money() {
        let bitcoin_amount = 1.0;

        let start = gen_start_state(bitcoin_amount, U256::from(10));
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

        let script = bitcoin_htlc_address(&swap).script_pubkey();

        let script_pub_key = ScriptPubKey {
            asm: String::from(""),
            hex: script.clone(),
            req_sigs: None,
            script_type: ScriptType::NullData,
            addresses: None,
        };

        let transaction_output = TransactionOutput {
            value: swap.clone().source_asset.bitcoin(),
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

        let result = BitcoinQuantity::is_contained_in_source_ledger_transaction(
            swap.clone(),
            bitcoin_transaction.clone(),
        );

        let txid = bitcoin_transaction.txid();
        let expected_outpoint = OutPoint { txid, vout: 0 };

        assert_that(&result).is_ok_containing(expected_outpoint)
    }

    #[test]
    fn bitcoin_transaction_does_not_contain_output() {
        let bitcoin_amount = 1.0;

        let start = gen_start_state(bitcoin_amount, U256::from(10));
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

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

        let result =
            BitcoinQuantity::is_contained_in_source_ledger_transaction(swap, transaction.into());

        assert_that(&result).is_err_containing(ValidationError::WrongTransaction)
    }

    #[test]
    fn bitcoin_transaction_does_not_contain_enough_money() {
        let bitcoin_amount = 1.0;

        let start = gen_start_state(bitcoin_amount, U256::from(10));
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

        let script = bitcoin_htlc_address(&swap).script_pubkey();
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

        let result =
            BitcoinQuantity::is_contained_in_source_ledger_transaction(swap, transaction.into());

        let expected_error = ValidationError::UnexpectedAsset {
            found: BitcoinQuantity::from_bitcoin(provided_bitcoin_amount),
            expected: BitcoinQuantity::from_bitcoin(bitcoin_amount),
        };

        assert_that(&result).is_err_containing(expected_error)
    }

    #[test]
    pub fn ethereum_tx_has_correct_funding_and_correct_data_should_return_contract_address() {
        let ether_amount = U256::from(10);

        let start = gen_start_state(1.0, ether_amount);
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

        let provided_ether_amount = U256::from(10);
        let ethereum_transaction = ethereum_support::Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0a81e8be41b21f651a71aab1a85c6813b8bbccf8".parse().unwrap(),
            to: None,
            value: provided_ether_amount,
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: ethereum_htlc(&swap).compile_to_hex().into(),
        };

        let expected_address =
            ethereum_support::Address::from_str("994a1e7928556ba81b85bf3c665a3f4a0f0d4cd9")
                .unwrap();

        let result =
            EtherQuantity::is_contained_in_target_ledger_transaction(swap, ethereum_transaction);

        assert_that(&result).is_ok_containing(expected_address)
    }

    #[test]
    pub fn ethereum_tx_has_incorrect_funding_and_correct_data_should_return_error() {
        let ether_amount = U256::from(10);

        let start = gen_start_state(1.0, ether_amount);
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

        let provided_ether_amount = U256::from(9);
        let ethereum_transaction = ethereum_support::Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0a81e8be41b21f651a71aab1a85c6813b8bbccf8".parse().unwrap(),
            to: None,
            value: provided_ether_amount,
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: ethereum_htlc(&swap).compile_to_hex().into(),
        };

        let result =
            EtherQuantity::is_contained_in_target_ledger_transaction(swap, ethereum_transaction);

        let expected_error = ValidationError::UnexpectedAsset {
            found: EtherQuantity::from_wei(provided_ether_amount),
            expected: EtherQuantity::from_wei(ether_amount),
        };

        assert_that(&result).is_err_containing(expected_error)
    }

    #[test]
    pub fn ethereum_tx_has_correct_funding_but_incorrect_data_should_return_error() {
        let ether_amount = U256::from(10);

        let start = gen_start_state(1.0, ether_amount);
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

        let provided_ether_amount = U256::from(9);
        let ethereum_transaction = ethereum_support::Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0a81e8be41b21f651a71aab1a85c6813b8bbccf8".parse().unwrap(),
            to: None,
            value: provided_ether_amount,
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![1, 2, 3]),
        };

        let result =
            EtherQuantity::is_contained_in_target_ledger_transaction(swap, ethereum_transaction);

        let expected_error = ValidationError::WrongTransaction;

        assert_that(&result).is_err_containing(expected_error)
    }

    #[test]
    pub fn ethereum_tx_has_correct_funding_but_not_sending_to_0_should_return_error() {
        let ether_amount = U256::from(10);

        let start = gen_start_state(1.0, ether_amount);
        let response = gen_response();
        let swap = OngoingSwap::new(start, response);

        let provided_ether_amount = U256::from(9);
        let ethereum_transaction = ethereum_support::Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0a81e8be41b21f651a71aab1a85c6813b8bbccf8".parse().unwrap(),
            to: Some("0000000000000000000000000000000000000001".parse().unwrap()),
            value: provided_ether_amount,
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: ethereum_htlc(&swap).compile_to_hex().into(),
        };

        let result =
            EtherQuantity::is_contained_in_target_ledger_transaction(swap, ethereum_transaction);

        let expected_error = ValidationError::WrongTransaction;

        assert_that(&result).is_err_containing(expected_error)
    }
}
