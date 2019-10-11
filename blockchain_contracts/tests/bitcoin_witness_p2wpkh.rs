pub mod bitcoin_helper;

use bitcoin_helper::{new_tc_bitcoincore_client, RegtestHelperClient};
use bitcoincore_rpc::RpcApi;
use blockchain_contracts::bitcoin::witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use rust_bitcoin::{
    consensus::encode::serialize_hex,
    secp256k1::{self, Secp256k1},
    Address, Amount, PrivateKey,
};
use spectral::prelude::*;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn redeem_single_p2wpkh() {
    let _ = pretty_env_logger::try_init();

    let secp: Secp256k1<secp256k1::All> = Secp256k1::new();
    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.mine_bitcoins();
    let input_amount = Amount::from_sat(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &private_key.key);
    let (_, outpoint) = client.create_p2wpkh_vout_at(public_key, input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            outpoint,
            input_amount,
            private_key.key.p2wpkh_unlock_parameters(&secp),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(&secp, fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex).unwrap();

    client.generate(1, None).unwrap();

    let actual_amount = client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .unwrap()
        .value;
    let expected_amount = (input_amount - fee).as_sat();

    assert_that(&actual_amount).is_equal_to(expected_amount);
}

#[test]
fn redeem_two_p2wpkh() {
    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    let secp: Secp256k1<secp256k1::All> = Secp256k1::new();

    client.mine_bitcoins();
    let input_amount = Amount::from_sat(100_000_001);

    let private_key_1 =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let secret_key_1 = private_key_1.key;
    let public_key_1 = secp256k1::PublicKey::from_secret_key(&secp, &secret_key_1);

    let private_key_2 =
        PrivateKey::from_str("L1dDXCRQuNuhinf5SHbAmNUncovqFdA6ozJP4mbT7Mg53tWFFMFL").unwrap();
    let secret_key_2 = private_key_2.key;
    let public_key_2 = secp256k1::PublicKey::from_secret_key(&secp, &secret_key_2);

    let (_, vout_1) = client.create_p2wpkh_vout_at(public_key_1, input_amount);
    let (_, vout_2) = client.create_p2wpkh_vout_at(public_key_2, input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                vout_1,
                input_amount,
                secret_key_1.p2wpkh_unlock_parameters(&secp),
            ),
            PrimedInput::new(
                vout_2,
                input_amount,
                secret_key_2.p2wpkh_unlock_parameters(&secp),
            ),
        ],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(&secp, fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex).unwrap();

    client.generate(1, None).unwrap();

    let actual_amount = client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .unwrap()
        .value;
    let expected_amount = Amount::from_sat(input_amount.as_sat() * 2 - fee.as_sat()).as_sat();

    assert_that(&actual_amount).is_equal_to(&expected_amount);
}
