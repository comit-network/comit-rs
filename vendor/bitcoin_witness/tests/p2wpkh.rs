use bitcoin_rpc_test_helpers::RegtestHelperClient;
use bitcoin_support::{serialize_hex, Address, BitcoinQuantity, PrivateKey};
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use bitcoincore_rpc::RpcApi;
use secp256k1_keypair::KeyPair;
use spectral::prelude::*;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn redeem_single_p2wpkh() {
    let _ = env_logger::try_init();

    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.mine_bitcoins();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let keypair: KeyPair = private_key.key.clone().into();

    let (_, outpoint) = client.create_p2wpkh_vout_at(keypair.public_key().clone(), input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            outpoint,
            input_amount,
            keypair.p2wpkh_unlock_parameters(),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex).unwrap();

    client.generate(1, None).unwrap();

    let actual_amount = client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .unwrap()
        .value;
    let expected_amount = (input_amount - fee).satoshi();

    assert_that(&actual_amount).is_equal_to(expected_amount);
}

#[test]
fn redeem_two_p2wpkh() {
    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);

    client.mine_bitcoins();
    let input_amount = BitcoinQuantity::from_satoshi(100_000_001);
    let private_key_1 =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let keypair_1: KeyPair = private_key_1.key.clone().into();
    let private_key_2 =
        PrivateKey::from_str("L1dDXCRQuNuhinf5SHbAmNUncovqFdA6ozJP4mbT7Mg53tWFFMFL").unwrap();
    let keypair_2: KeyPair = private_key_2.key.clone().into();

    let (_, vout_1) = client.create_p2wpkh_vout_at(keypair_1.public_key().clone(), input_amount);
    let (_, vout_2) = client.create_p2wpkh_vout_at(keypair_2.public_key().clone(), input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(vout_1, input_amount, keypair_1.p2wpkh_unlock_parameters()),
            PrimedInput::new(vout_2, input_amount, keypair_2.p2wpkh_unlock_parameters()),
        ],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex).unwrap();

    client.generate(1, None).unwrap();

    let actual_amount = client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .unwrap()
        .value;
    let expected_amount =
        BitcoinQuantity::from_satoshi(input_amount.satoshi() * 2 - fee.satoshi()).satoshi();

    assert_that(&actual_amount).is_equal_to(&expected_amount);
}
