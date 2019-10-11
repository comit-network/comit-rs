use bitcoin_rpc_test_helpers::RegtestHelperClient;
use bitcoin_support::{serialize_hex, Address, Amount, PrivateKey};
use bitcoin_witness::{
    secp256k1::{PublicKey, Secp256k1},
    PrimedInput, PrimedTransaction, UnlockP2wpkh,
};
use bitcoincore_rpc::RpcApi;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn sign_with_rate() {
    let _ = env_logger::try_init();
    let docker = Cli::default();
    let secp = Secp256k1::new();

    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.mine_bitcoins();
    let input_amount = Amount::from_sat(100_000_001);
    let private_key =
        PrivateKey::from_str("L4nZrdzNnawCtaEcYGWuPqagQA3dJxVPgN8ARTXaMLCxiYCy89wm").unwrap();
    let secret_key = private_key.key;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

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
