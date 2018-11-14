extern crate bitcoin_rpc_client;
extern crate bitcoin_rpc_test_helpers;
extern crate bitcoin_support;
extern crate bitcoin_witness;
extern crate env_logger;
extern crate hex;
extern crate secp256k1_support;
extern crate spectral;
extern crate tc_bitcoincore_client;
extern crate testcontainers;

use bitcoin_rpc_client::*;
use bitcoin_rpc_test_helpers::RegtestHelperClient;
use bitcoin_support::{serialize::serialize_hex, Address, BitcoinQuantity, OutPoint, PrivateKey};
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use secp256k1_support::KeyPair;
use spectral::prelude::*;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn redeem_single_p2wpkh() {
    let _ = env_logger::try_init();

    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.enable_segwit();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let keypair: KeyPair = private_key.secret_key().clone().into();

    let (txid, vout) = client.create_p2wpkh_vout_at(keypair.public_key().clone(), input_amount);

    let alice_addr: Address = client.get_new_address().unwrap().unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            OutPoint { txid, vout: vout.n },
            input_amount,
            keypair.p2wpkh_unlock_parameters(),
        )],
        output_address: alice_addr.clone(),
        locktime: 0,
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .unwrap();

    client.generate(1).unwrap().unwrap();

    let actual_amount = client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .unwrap()
        .amount;
    let expected_amount = (input_amount - fee).bitcoin();

    assert_that(&actual_amount).is_close_to(expected_amount, 0.000_000_01);
}

#[test]
fn redeem_two_p2wpkh() {
    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);

    client.enable_segwit();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key_1 =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let keypair_1: KeyPair = private_key_1.secret_key().clone().into();
    let private_key_2 =
        PrivateKey::from_str("L1dDXCRQuNuhinf5SHbAmNUncovqFdA6ozJP4mbT7Mg53tWFFMFL").unwrap();
    let keypair_2: KeyPair = private_key_2.secret_key().clone().into();

    let (txid_1, vout_1) =
        client.create_p2wpkh_vout_at(keypair_1.public_key().clone(), input_amount);
    let (txid_2, vout_2) =
        client.create_p2wpkh_vout_at(keypair_2.public_key().clone(), input_amount);

    let alice_addr: Address = client.get_new_address().unwrap().unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                OutPoint {
                    txid: txid_1,
                    vout: vout_1.n,
                },
                input_amount,
                keypair_1.p2wpkh_unlock_parameters(),
            ),
            PrimedInput::new(
                OutPoint {
                    txid: txid_2,
                    vout: vout_2.n,
                },
                input_amount,
                keypair_2.p2wpkh_unlock_parameters(),
            ),
        ],
        output_address: alice_addr.clone(),
        locktime: 0,
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .unwrap();

    client.generate(1).unwrap().unwrap();

    let actual_amount = client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .unwrap()
        .amount;
    let expected_amount =
        BitcoinQuantity::from_satoshi(input_amount.satoshi() * 2 - fee.satoshi()).bitcoin();

    assert_that(&actual_amount).is_close_to(&expected_amount, 0.000_000_01);
}
