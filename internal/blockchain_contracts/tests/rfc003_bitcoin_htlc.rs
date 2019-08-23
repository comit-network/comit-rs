#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin_helper;
pub mod ethereum_helper;
pub mod htlc_harness;
pub mod parity_client;

use crate::{
    bitcoin_helper::RegtestHelperClient,
    htlc_harness::{Timestamp, SECRET, SECRET_HASH},
};
use bitcoin::{
    consensus::encode::serialize_hex, network::constants::Network, Address, OutPoint, PrivateKey,
};
use bitcoin_quantity::BitcoinQuantity;
use bitcoincore_rpc::RpcApi;
use blockchain_contracts::bitcoin::{
    pubkey_hash::PubkeyHash,
    rfc003::{BitcoinHtlc, UnlockStrategy},
};
use secp256k1::{PublicKey, SecretKey};
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

struct HtlcSetup {
    location: OutPoint,
    amount: BitcoinQuantity,
    htlc: BitcoinHtlc,
    refund_timestamp: Timestamp,
    redeem_secret_key: SecretKey,
    refund_secret_key: SecretKey,
}

fn fund_htlc(client: &bitcoincore_rpc::Client, secret_hash: [u8; 32]) -> HtlcSetup {
    let redeem_privkey =
        PrivateKey::from_str("cSrWvMrWE3biZinxPZc1hSwMMEdYgYsFpB6iEoh8KraLqYZUUCtt").unwrap();
    let redeem_secret_key = redeem_privkey.key;
    let redeem_pubkey_hash: PubkeyHash =
        PublicKey::from_secret_key(&*blockchain_contracts::SECP, &redeem_secret_key).into();

    let refund_privkey =
        PrivateKey::from_str("cNZUJxVXghSri4dUaNW8ES3KiFyDoWVffLYDz7KMcHmKhLdFyZPx").unwrap();
    let refund_secret_key = refund_privkey.key;
    let refund_pubkey_hash: PubkeyHash =
        PublicKey::from_secret_key(&*blockchain_contracts::SECP, &refund_secret_key).into();

    let current_time = client.get_blockchain_info().unwrap().mediantime;
    let current_time = u32::try_from(current_time).unwrap();
    let refund_timestamp = Timestamp::from(current_time).plus(5);
    let amount = BitcoinQuantity::from_satoshi(100_000_001);

    let htlc = BitcoinHtlc::new(
        refund_timestamp.into(),
        redeem_pubkey_hash.into(),
        refund_pubkey_hash.into(),
        secret_hash,
    );

    let htlc_address = htlc.compute_address(Network::Regtest);

    let txid = client
        .send_to_address(
            &htlc_address.clone(),
            amount.bitcoin(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    client.generate(1, None).unwrap();

    let location = client.find_vout_for_address(&txid, &htlc_address);

    HtlcSetup {
        location,
        amount,
        htlc,
        refund_timestamp,
        redeem_secret_key,
        refund_secret_key,
    }
}

#[test]
fn redeem_htlc_with_secret() {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();

    let container = docker.run(BitcoinCore::default());
    let client = new_tc_bitcoincore_client(&container);
    client.generate(101, None).unwrap();

    let HtlcSetup {
        location,
        amount: input_amount,
        htlc,
        redeem_secret_key,
        ..
    } = fund_htlc(&client, SECRET_HASH);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let redeem_tx = htlc
        .unlock(
            location,
            input_amount.satoshi(),
            alice_addr.clone(),
            20,
            UnlockStrategy::Redeem {
                key: redeem_secret_key,
                secret: *SECRET,
            },
        )
        .unwrap();
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

    let HtlcSetup {
        location,
        amount: input_amount,
        htlc,
        refund_secret_key,
        refund_timestamp,
        ..
    } = fund_htlc(&client, SECRET_HASH);

    let alice_addr: Address = client.get_new_address(None, None).unwrap();

    let refund_tx = htlc
        .unlock(
            location,
            input_amount.satoshi(),
            alice_addr.clone(),
            10,
            UnlockStrategy::Refund {
                key: refund_secret_key,
            },
        )
        .unwrap();
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
