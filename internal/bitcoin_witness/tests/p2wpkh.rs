use bitcoin_rpc_test_helpers::RegtestHelperClient;
use bitcoin_support::{serialize_hex, Address, Amount, PrivateKey};
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use bitcoincore_rpc::RpcApi;
use secp256k1::Secp256k1;
use secp256k1_omni_context::{secp256k1, Builder};
use spectral::prelude::*;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn redeem_single_p2wpkh() {
    let _ = env_logger::try_init();

    let secp: Secp256k1<secp256k1::All> = Secp256k1::new();
    let docker = Cli::default();
    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.mine_bitcoins();
    let input_amount = Amount::from_sat(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let secret_key = Builder::new(secp.clone())
        .secret_key(private_key.key)
        .build()
        .unwrap();

    let (_, outpoint) = client.create_p2wpkh_vout_at(secret_key.public_key(), input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            outpoint,
            input_amount,
            secret_key.clone().p2wpkh_unlock_parameters(),
        )],
        output_address: alice_addr.clone(),
        secp,
    }
    .sign_with_fee(fee);

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
    let client = tc_bitcoincore_client::new(&container);
    let secp: Secp256k1<secp256k1::All> = Secp256k1::new();

    client.mine_bitcoins();
    let input_amount = Amount::from_sat(100_000_001);
    let private_key_1 =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let secret_key_1 = Builder::new(secp.clone())
        .secret_key(private_key_1.key)
        .build()
        .unwrap();
    let private_key_2 =
        PrivateKey::from_str("L1dDXCRQuNuhinf5SHbAmNUncovqFdA6ozJP4mbT7Mg53tWFFMFL").unwrap();
    let secret_key_2 = Builder::new(secp.clone())
        .secret_key(private_key_2.key)
        .build()
        .unwrap();

    let (_, vout_1) = client.create_p2wpkh_vout_at(secret_key_1.clone().public_key(), input_amount);
    let (_, vout_2) = client.create_p2wpkh_vout_at(secret_key_2.clone().public_key(), input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![
            PrimedInput::new(
                vout_1,
                input_amount,
                secret_key_1.p2wpkh_unlock_parameters(),
            ),
            PrimedInput::new(
                vout_2,
                input_amount,
                secret_key_2.p2wpkh_unlock_parameters(),
            ),
        ],
        output_address: alice_addr.clone(),
        secp,
    }
    .sign_with_fee(fee);

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
