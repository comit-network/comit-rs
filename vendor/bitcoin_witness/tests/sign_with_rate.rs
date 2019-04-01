use bitcoin_rpc_client::*;
use bitcoin_rpc_test_helpers::RegtestHelperClient;
use bitcoin_support::{serialize::serialize_hex, Address, BitcoinQuantity, OutPoint, PrivateKey};
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockP2wpkh};
use secp256k1_support::KeyPair;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

#[test]
fn sign_with_rate() {
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

	let rate = 42.0;

	let primed_tx = PrimedTransaction {
		inputs: vec![PrimedInput::new(
			OutPoint { txid, vout: vout.n },
			input_amount,
			keypair.p2wpkh_unlock_parameters(),
		)],
		output_address: alice_addr.clone(),
	};

	let redeem_tx = primed_tx.sign_with_rate(rate).unwrap();

	let redeem_tx_hex = serialize_hex(&redeem_tx).unwrap();

	let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

	let rpc_redeem_txid = client
		.send_raw_transaction(raw_redeem_tx.clone())
		.unwrap()
		.unwrap();

	client.generate(1).unwrap().unwrap();

	assert!(client
		.find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
		.is_some())
}
