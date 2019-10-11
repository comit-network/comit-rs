pub mod bitcoin_helper;

use crate::bitcoin_helper::new_tc_bitcoincore_client;
use bitcoin_helper::RegtestHelperClient;
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use bitcoincore_rpc::RpcApi;
use rust_bitcoin::{
    consensus::encode::serialize_hex,
    secp256k1::{self, Secp256k1},
    Address, Amount, PrivateKey,
};
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn sign_with_rate() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let secp = Secp256k1::new();

    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.generate(101, None).unwrap();
    let input_amount = Amount::from_sat(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let secret_key = private_key.key;
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    let (_, outpoint) = client.create_p2wpkh_vout_at(public_key, input_amount);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let rate = 42;

    let primed_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            outpoint,
            input_amount,
            secret_key.p2wpkh_unlock_parameters(&secp),
        )],
        output_address: alice_addr.clone(),
    };

    let redeem_tx = primed_tx.sign_with_rate(&secp, rate).unwrap();

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex).unwrap();

    client.generate(1, None).unwrap();

    assert!(client
        .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
        .is_some())
}
