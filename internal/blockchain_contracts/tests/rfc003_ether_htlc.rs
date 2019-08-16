#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod ethereum_helper;
pub mod htlc_harness;
pub mod parity_client;

use crate::htlc_harness::{
    ether_harness, sleep_until, CustomSizeSecret, EtherHarnessParams, Timestamp, SECRET,
};
use spectral::prelude::*;
use testcontainers::clients::Cli;
use web3::types::{Bytes, H256, U256};

// keccak256(Redeemed())
const REDEEMED_LOG_MSG: &str = "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413";
// keccak256(Refunded())
const REFUNDED_LOG_MSG: &str = "0x5D26862916391BF49478B2F5103B0720A842B45EF145A268F2CD1FB2AED55178";

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let docker = Cli::default();
    let (_alice, bob, htlc, client, _handle, _container) =
        ether_harness(&docker, EtherHarnessParams::default());

    assert_eq!(client.eth_balance_of(bob), U256::from(0));

    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );

    // Send correct secret to contract
    client.send_data(htlc, Some(Bytes(SECRET.to_vec())));

    assert_eq!(
        client.eth_balance_of(bob),
        U256::from("0400000000000000000")
    );
    assert_eq!(client.eth_balance_of(htlc), U256::from(0));
}

#[test]
fn given_deployed_htlc_when_refunded_after_expiry_time_then_money_is_refunded() {
    let docker = Cli::default();
    let harness_params = EtherHarnessParams::default();
    let (_alice, bob, htlc, client, _handle, _container) =
        ether_harness(&docker, harness_params.clone());

    assert_eq!(client.eth_balance_of(bob), U256::from(0));
    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );

    // Wait for the contract to expire
    sleep_until(harness_params.htlc_refund_timestamp);
    client.send_data(htlc, None);

    assert_eq!(client.eth_balance_of(bob), U256::from(0));
    assert_eq!(client.eth_balance_of(htlc), U256::from(0));
}

#[test]
fn given_htlc_and_refund_before_expiry_nothing_happens() {
    let docker = Cli::default();
    let (_alice, bob, htlc, client, _handle, _container) =
        ether_harness(&docker, EtherHarnessParams {
            htlc_refund_timestamp: Timestamp::now().plus(1000000),
            ..Default::default()
        });

    assert_eq!(client.eth_balance_of(bob), U256::from(0));
    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );

    // Don't wait for the timeout and don't send a secret
    client.send_data(htlc, None);

    assert_eq!(client.eth_balance_of(bob), U256::from(0));
    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );
}

#[test]
fn given_htlc_and_redeem_should_emit_redeem_log_msg_with_secret() {
    let docker = Cli::default();
    let (_alice, _bob, htlc, client, _handle, _container) =
        ether_harness(&docker, EtherHarnessParams::default());

    // Send incorrect secret to contract
    let transaction_receipt = client.send_data(htlc, Some(Bytes(b"I'm a h4x0r".to_vec())));
    assert_that(&transaction_receipt.logs).has_length(0);

    // Send correct secret to contract
    let transaction_receipt = client.send_data(htlc, Some(Bytes(SECRET.to_vec())));

    assert_that(&transaction_receipt.logs).has_length(1);
    let topic: H256 = REDEEMED_LOG_MSG.into();
    assert_that(&transaction_receipt.logs[0].topics).has_length(1);
    assert_that(&transaction_receipt.logs[0].topics).contains(topic);
    assert_that(&transaction_receipt.logs[0].data).is_equal_to(Bytes(SECRET.to_vec()));
}

#[test]
fn given_htlc_and_refund_should_emit_refund_log_msg() {
    let docker = Cli::default();
    let harness_params = EtherHarnessParams::default();
    let (_alice, _bob, htlc, client, _handle, _container) =
        ether_harness(&docker, harness_params.clone());

    // Wait for the timelock to expire
    sleep_until(harness_params.htlc_refund_timestamp);
    let transaction_receipt = client.send_data(htlc, None);

    assert_that(&transaction_receipt.logs).has_length(1);
    let topic: H256 = REFUNDED_LOG_MSG.into();
    assert_that(&transaction_receipt.logs[0].topics).has_length(1);
    assert_that(&transaction_receipt.logs[0].topics).contains(topic);
    assert_that(&transaction_receipt.logs[0].data).is_equal_to(Bytes(vec![]));
}

#[test]
fn given_deployed_htlc_when_redeem_with_short_secret_then_ether_should_not_be_transferred() {
    let docker = Cli::default();
    let secret = CustomSizeSecret(vec![
        1u8, 2u8, 3u8, 4u8, 6u8, 6u8, 7u8, 9u8, 10u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    ]);

    let (_alice, bob, htlc, client, _handle, _container) = ether_harness(
        &docker,
        EtherHarnessParams::default().with_secret_hash(secret.hash()),
    );

    assert_eq!(client.eth_balance_of(bob), U256::from(0));

    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );

    client.send_data(
        htlc,
        Some(Bytes(vec![1u8, 2u8, 3u8, 4u8, 6u8, 6u8, 7u8, 9u8, 10u8])),
    );

    assert_eq!(client.eth_balance_of(bob), U256::from(0));

    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );
}

#[test]
fn given_correct_zero_secret_htlc_should_redeem() {
    let docker = Cli::default();
    let secret_vec = vec![
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    ];
    let secret = CustomSizeSecret(secret_vec.clone());

    let (_alice, bob, htlc, client, _handle, _container) = ether_harness(
        &docker,
        EtherHarnessParams::default().with_secret_hash(secret.hash()),
    );

    assert_eq!(client.eth_balance_of(bob), U256::from(0));

    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );

    client.send_data(htlc, Some(Bytes(secret_vec)));

    assert_eq!(
        client.eth_balance_of(bob),
        U256::from("0400000000000000000")
    );

    assert_eq!(client.eth_balance_of(htlc), U256::from(0));
}

#[test]
fn given_short_zero_secret_htlc_should_not_redeem() {
    let docker = Cli::default();
    let secret = CustomSizeSecret(vec![
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
    ]);

    let (_alice, bob, htlc, client, _handle, _container) = ether_harness(
        &docker,
        EtherHarnessParams::default().with_secret_hash(secret.hash()),
    );

    assert_eq!(client.eth_balance_of(bob), U256::from(0));

    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );

    client.send_data(
        htlc,
        Some(Bytes(vec![
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ])),
    );

    assert_eq!(client.eth_balance_of(bob), U256::from(0));

    assert_eq!(
        client.eth_balance_of(htlc),
        U256::from("0400000000000000000")
    );
}
