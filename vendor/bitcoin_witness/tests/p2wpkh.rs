extern crate bitcoin_rpc;
extern crate bitcoin_rpc_helpers;
extern crate bitcoin_support;
extern crate bitcoin_witness;
extern crate coblox_bitcoincore;
extern crate env_logger;
extern crate hex;
extern crate secp256k1_support;
extern crate testcontainers;

use bitcoin_rpc::{BitcoinCoreClient, BitcoinRpcApi};
use bitcoin_rpc_helpers::RegtestHelperClient;
use bitcoin_support::{serialize::serialize_hex, Address, BitcoinQuantity, PrivateKey};
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use secp256k1_support::KeyPair;
use std::str::FromStr;

use coblox_bitcoincore::BitcoinCore;
use testcontainers::{clients::DockerCli, Node};

#[test]
fn redeem_single_p2wpkh() {
    let _ = env_logger::try_init();

    let node = Node::<BitcoinCoreClient, DockerCli>::new::<BitcoinCore>();
    let client = node.get_client();
    client.enable_segwit();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let keypair: KeyPair = private_key.secret_key().clone().into();

    let (txid, vout) = client.create_p2wpkh_vout_at(keypair.public_key().clone(), input_amount);

    let alice_addr: Address = client
        .get_new_address()
        .unwrap()
        .into_result()
        .unwrap()
        .into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            txid.into(),
            vout.n,
            input_amount,
            keypair.p2wpkh_unlock_parameters(),
        )],
        output_address: alice_addr.clone(),
        locktime: 0,
    }.sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .into_result()
        .unwrap();

    client.generate(1).unwrap();

    assert_eq!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .unwrap()
            .amount,
        (input_amount - fee).bitcoin(),
        "utxo should exist after redeeming p2wpkhoutput"
    );
}

#[test]
fn redeem_two_p2wpkh() {
    let node = Node::<BitcoinCoreClient, DockerCli>::new::<BitcoinCore>();
    let client = node.get_client();

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

    let alice_addr: Address = client
        .get_new_address()
        .unwrap()
        .into_result()
        .unwrap()
        .into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                txid_1.into(),
                vout_1.n,
                input_amount,
                keypair_1.p2wpkh_unlock_parameters(),
            ),
            PrimedInput::new(
                txid_2.into(),
                vout_2.n,
                input_amount,
                keypair_2.p2wpkh_unlock_parameters(),
            ),
        ],
        output_address: alice_addr.clone(),
        locktime: 0,
    }.sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

    let raw_redeem_tx = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx_hex.as_str());

    let rpc_redeem_txid = client
        .send_raw_transaction(raw_redeem_tx.clone())
        .unwrap()
        .into_result()
        .unwrap();

    client.generate(1).unwrap();

    assert_eq!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .unwrap()
            .amount,
        BitcoinQuantity::from_satoshi(input_amount.satoshi() * 2 - fee.satoshi()).bitcoin(),
        "The utxo should include the amounts from both inputs"
    );
}
