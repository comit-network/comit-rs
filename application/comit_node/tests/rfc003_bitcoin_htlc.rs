#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

pub mod ethereum_wallet;
pub mod htlc_harness;
pub mod parity_client;

use crate::htlc_harness::{sleep_until, CustomSizeSecret};
use bitcoin_rpc_client::*;
use bitcoin_rpc_test_helpers::RegtestHelperClient;
use bitcoin_support::{
    serialize_hex, Address, BitcoinQuantity, Network, OutPoint, PrivateKey, PubkeyHash,
};
use bitcoin_witness::{
    PrimedInput, PrimedTransaction, UnlockParameters, Witness, SEQUENCE_ALLOW_NTIMELOCK_NO_RBF,
};
use comit_node::swap_protocols::rfc003::{bitcoin::Htlc, Secret, SecretHash, Timestamp};
use secp256k1_support::KeyPair;
use spectral::prelude::*;
use std::str::FromStr;
use testcontainers::{clients::Cli, images::coblox_bitcoincore::BitcoinCore, Docker};

impl CustomSizeSecret {
    pub fn unlock_with_secret(self, htlc: &Htlc, keypair: KeyPair) -> UnlockParameters {
        let public_key = keypair.public_key();
        UnlockParameters {
            witness: vec![
                Witness::Signature(keypair),
                Witness::PublicKey(public_key),
                Witness::Data(self.0),
                Witness::Bool(true),
                Witness::PrevScript,
            ],
            locktime: 0,
            sequence: SEQUENCE_ALLOW_NTIMELOCK_NO_RBF,
            prev_script: htlc.script().clone(),
        }
    }
}

fn fund_htlc(
    client: &BitcoinCoreClient,
    secret_hash: SecretHash,
) -> (
    TransactionId,
    rpc::TransactionOutput,
    BitcoinQuantity,
    Htlc,
    Timestamp,
    KeyPair,
    KeyPair,
) {
    let redeem_privkey =
        PrivateKey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();
    let redeem_keypair: KeyPair = redeem_privkey.key.clone().into();
    let redeem_pubkey_hash: PubkeyHash = redeem_keypair.public_key().clone().into();
    let refund_privkey =
        PrivateKey::from_str("cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx").unwrap();
    let refund_keypair: KeyPair = refund_privkey.key.clone().into();
    let refund_pubkey_hash: PubkeyHash = refund_keypair.public_key().clone().into();
    let refund_timestamp = Timestamp::now().plus(10);
    let amount = BitcoinQuantity::from_satoshi(100_000_001);

    let htlc = Htlc::new(
        redeem_pubkey_hash,
        refund_pubkey_hash,
        secret_hash,
        refund_timestamp,
    );

    let htlc_address = htlc.compute_address(Network::Regtest);

    let txid = client
        .send_to_address(&htlc_address.clone().into(), amount.bitcoin())
        .unwrap()
        .unwrap();

    client.generate(1).unwrap().unwrap();

    let vout = client.find_vout_for_address(&txid, &htlc_address);

    (
        txid,
        vout.clone(),
        amount,
        htlc,
        refund_timestamp,
        redeem_keypair,
        refund_keypair,
    )
}

#[test]
fn redeem_htlc_with_secret() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.generate(432).unwrap().unwrap();

    let secret = Secret::from(*b"hello world, you are beautiful!!");
    let (txid, vout, input_amount, htlc, _, keypair, _) = fund_htlc(&client, secret.hash());

    assert!(
        htlc.can_be_unlocked_with(secret, keypair).is_ok(),
        "Should be unlockable with the given secret and secret_key"
    );

    let alice_addr: Address = client.get_new_address().unwrap().unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            OutPoint { txid, vout: vout.n },
            input_amount,
            htlc.unlock_with_secret(keypair, &secret),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

    let rpc_redeem_txid = client.send_raw_transaction(raw_redeem_tx).unwrap().unwrap();

    client.generate(1).unwrap().unwrap();

    assert!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .is_some(),
        "utxo should exist after redeeming htlc"
    );
}

#[test]
fn redeem_refund_htlc() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.generate(432).unwrap().unwrap();

    let secret = Secret::from(*b"hello world, you are beautiful!!");
    let (txid, vout, input_amount, htlc, refund_timestamp, _, keypair) =
        fund_htlc(&client, secret.hash());

    let alice_addr: Address = client.get_new_address().unwrap().unwrap().into();
    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            OutPoint { txid, vout: vout.n },
            input_amount,
            htlc.unlock_after_timeout(keypair),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);
    let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

    let rpc_redeem_txid = client.send_raw_transaction(raw_redeem_tx).unwrap().unwrap();
    client.generate(1).unwrap().unwrap();

    assert!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .is_none(),
        "utxo should not yet exist"
    );

    sleep_until(refund_timestamp);

    client.generate(1).unwrap().unwrap();

    assert!(
        client
            .find_utxo_at_tx_for_address(&rpc_redeem_txid, &alice_addr)
            .is_some(),
        "utxo should exist after refunding htlc"
    );
}

#[test]
fn redeem_htlc_with_long_secret() -> Result<(), failure::Error> {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.generate(432).unwrap().unwrap();

    let secret = CustomSizeSecret::from_str("Grandmother, what big secret you have!")?;
    assert_eq!(secret.0.len(), 38);

    let (txid, vout, input_amount, htlc, _, keypair, _) = fund_htlc(&client, secret.hash());

    let alice_addr: Address = client.get_new_address().unwrap().unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            OutPoint { txid, vout: vout.n },
            input_amount,
            secret.unlock_with_secret(&htlc, keypair),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

    let rpc_redeem_txid = client.send_raw_transaction(raw_redeem_tx).unwrap();

    assert_that(&rpc_redeem_txid).is_err_containing(&RpcError {
        code: -26,
        message:
            "non-mandatory-script-verify-flag (Script failed an OP_EQUALVERIFY operation) (code 64)"
                .to_string(),
    });

    Ok(())
}

#[test]
fn redeem_htlc_with_short_secret() -> Result<(), failure::Error> {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = tc_bitcoincore_client::new(&container);
    client.generate(432).unwrap().unwrap();

    let secret = CustomSizeSecret::from_str("teeny-weeny-bunny")?;
    assert_eq!(secret.0.len(), 17);

    let (txid, vout, input_amount, htlc, _, keypair, _) = fund_htlc(&client, secret.hash());

    let alice_addr: Address = client.get_new_address().unwrap().unwrap().into();

    let fee = BitcoinQuantity::from_satoshi(1000);

    let redeem_tx = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            OutPoint { txid, vout: vout.n },
            input_amount,
            secret.unlock_with_secret(&htlc, keypair),
        )],
        output_address: alice_addr.clone(),
    }
    .sign_with_fee(fee);

    let redeem_tx_hex = serialize_hex(&redeem_tx);

    let raw_redeem_tx = rpc::SerializedRawTransaction(redeem_tx_hex);

    let rpc_redeem_txid = client.send_raw_transaction(raw_redeem_tx).unwrap();

    assert_that(&rpc_redeem_txid).is_err_containing(&RpcError {
        code: -26,
        message:
            "non-mandatory-script-verify-flag (Script failed an OP_EQUALVERIFY operation) (code 64)"
                .to_string(),
    });
    Ok(())
}
