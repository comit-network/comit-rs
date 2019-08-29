#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin_helper;
pub mod ethereum_helper;
pub mod htlc_harness;
pub mod parity_client;

use crate::{
    bitcoin_helper::RegtestHelperClient,
    htlc_harness::{CustomSizeSecret, Timestamp, SECRET, SECRET_HASH},
};
use bitcoin_witness::{PrimedInput, PrimedTransaction, UnlockParameters, Witness};
use bitcoincore_rpc::RpcApi;
use blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::BitcoinHtlc;
use rust_bitcoin::{
    consensus::encode::serialize_hex,
    hashes::{hash160, sha256d, Hash},
    network::constants::Network,
    secp256k1::{PublicKey, SecretKey},
    Address, Amount, OutPoint, PrivateKey,
};
use spectral::prelude::*;
use std::{convert::TryFrom, str::FromStr, thread::sleep, time::Duration};
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Container, Docker};

pub fn new_tc_bitcoincore_client<D: Docker>(
    container: &Container<'_, D, BitcoinCore>,
) -> bitcoincore_rpc::Client {
    let port = container.get_host_port(18443).unwrap();
    let auth = container.image().auth();

    let endpoint = format!("http://localhost:{}", port);

    bitcoincore_rpc::Client::new(
        endpoint,
        bitcoincore_rpc::Auth::UserPass(auth.username().to_owned(), auth.password().to_owned()),
    )
    .unwrap()
}

/// Mimic the functionality of [`BitcoinHtlc#unlock_with_secret`](method)
/// except that we want to insert our "CustomSizeSecret" on the witness
/// stack.
///
/// [method]: blockchain_contracts::bitcoin::rfc003::bitcoin_htlc::
/// BitcoinHtlc#unlock_with_secret
fn unlock_with_custom_size_secret(
    htlc: BitcoinHtlc,
    secret_key: SecretKey,
    custom_size_secret: CustomSizeSecret,
) -> UnlockParameters {
    let placeholder_secret = [0u8; 32];
    // First, unlock the HTLC with a placeholder secret
    let parameters = htlc.unlock_with_secret(secret_key, placeholder_secret);

    let UnlockParameters {
        mut witness,
        sequence,
        locktime,
        prev_script,
    } = parameters;

    // Secret for the secret in the witness stack (it is the only data) and replace
    // it with our custom size secret
    for w in &mut witness {
        if let Witness::Data(ref mut placeholder_secret) = w {
            placeholder_secret.clear();
            placeholder_secret.extend_from_slice(&custom_size_secret.0);
        }
    }

    // Return the patched `UnlockParameters`
    UnlockParameters {
        witness,
        locktime,
        sequence,
        prev_script,
    }
}

fn pubkey_hash(secret_key: &SecretKey) -> hash160::Hash {
    let public_key = PublicKey::from_secret_key(&*blockchain_contracts::SECP, &secret_key);

    hash160::Hash::hash(&public_key.serialize())
}

fn fund_htlc(
    client: &bitcoincore_rpc::Client,
    secret_hash: [u8; 32],
) -> (
    sha256d::Hash,
    OutPoint,
    Amount,
    BitcoinHtlc,
    Timestamp,
    SecretKey,
    SecretKey,
) {
    let redeem_privkey =
        PrivateKey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();
    let refund_privkey =
        PrivateKey::from_str("cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx").unwrap();
    let redeem_pubkey_hash = pubkey_hash(&redeem_privkey.key);
    let refund_pubkey_hash = pubkey_hash(&refund_privkey.key);

    let current_time = client.get_blockchain_info().unwrap().mediantime;
    let current_time = u32::try_from(current_time).unwrap();
    let refund_timestamp = Timestamp::from(current_time).plus(5);
    let amount = Amount::from_sat(100_000_001);

    let htlc = BitcoinHtlc::new(
        refund_timestamp.into(),
        refund_pubkey_hash,
        redeem_pubkey_hash,
        secret_hash,
    );

    let htlc_address = htlc.compute_address(Network::Regtest);

    let txid = client
        .send_to_address(
            &htlc_address.clone(),
            amount,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    client.generate(1, None).unwrap();

    let vout = client.find_vout_for_address(&txid, &htlc_address);

    (
        txid,
        vout,
        amount,
        htlc,
        refund_timestamp,
        redeem_privkey.key,
        refund_privkey.key,
    )
}

#[test]
fn redeem_htlc_with_secret() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.generate(101, None).unwrap();

    let (_, vout, input_amount, htlc, _, keypair, _) = fund_htlc(&client, SECRET_HASH);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            vout,
            input_amount,
            htlc.unlock_with_secret(keypair, SECRET.clone()),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex).unwrap();

    client.generate(1, None).unwrap();

    assert!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .is_some(),
        "utxo should exist after redeeming htlc"
    );
}

#[test]
fn refund_htlc() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.generate(101, None).unwrap();

    let (_, vout, input_amount, htlc, refund_timestamp, _, keypair) =
        fund_htlc(&client, SECRET_HASH);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();
    let fee = Amount::from_sat(1000);

    let refund_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            vout,
            input_amount,
            htlc.unlock_after_timeout(keypair),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let refund_tx_hex = serialize_hex(&refund_tx);

    let error = client
        .send_raw_transaction(refund_tx_hex.clone())
        .unwrap_err();

    // Can't access the type `RpcError`: https://github.com/rust-bitcoin/rust-bitcoincore-rpc/issues/50
    assert_eq!(
        format!("{:?}", error),
        "JsonRpc(Rpc(RpcError { code: -26, message: \"non-final (code 64)\", data: None }))"
    );

    loop {
        let time = client.get_blockchain_info().unwrap().mediantime;

        if time > u64::from(refund_timestamp) {
            break;
        }

        sleep(Duration::from_millis(2000));
        client.generate(1, None).unwrap();
    }

    let rpc_refund_txid = client.send_raw_transaction(refund_tx_hex.clone()).unwrap();
    client.generate(1, None).unwrap();

    assert!(
        client
            .find_utxo_at_tx_for_address(&rpc_refund_txid, &alice_addr)
            .is_some(),
        "utxo should exist after refunding htlc"
    );
}

#[test]
fn redeem_htlc_with_long_secret() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.generate(101, None).unwrap();

    let secret = CustomSizeSecret::from_str("Grandmother, what big secret you have!").unwrap();
    assert_eq!(secret.0.len(), 38);

    let (_, vout, input_amount, htlc, _, keypair, _) = fund_htlc(&client, secret.hash());

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            vout,
            input_amount,
            unlock_with_custom_size_secret(htlc, keypair, secret),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex);

    let error = assert_that(&rpc_redeem_txid).is_err().subject;

    // Can't access the type `RpcError`: https://github.com/rust-bitcoin/rust-bitcoincore-rpc/issues/50
    assert_eq!(
        format!("{:?}", error),
        "JsonRpc(Rpc(RpcError { code: -26, message: \"non-mandatory-script-verify-flag (Script failed an OP_EQUALVERIFY operation) (code 64)\", data: None }))"
    );
}

#[test]
fn redeem_htlc_with_short_secret() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.generate(101, None).unwrap();

    let secret = CustomSizeSecret::from_str("teeny-weeny-bunny").unwrap();
    assert_eq!(secret.0.len(), 17);

    let (_, vout, input_amount, htlc, _, keypair, _) = fund_htlc(&client, secret.hash());

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let fee = Amount::from_sat(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            vout,
            input_amount,
            unlock_with_custom_size_secret(htlc, keypair, secret),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let rpc_redeem_txid = client.send_raw_transaction(redeem_tx_hex);

    let error = assert_that(&rpc_redeem_txid).is_err().subject;

    // Can't access the type `RpcError`: https://github.com/rust-bitcoin/rust-bitcoincore-rpc/issues/50
    assert_eq!(
        format!("{:?}", error),
        "JsonRpc(Rpc(RpcError { code: -26, message: \"non-mandatory-script-verify-flag (Script failed an OP_EQUALVERIFY operation) (code 64)\", data: None }))"
    );
}
