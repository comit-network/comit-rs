#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

pub mod ethereum_wallet;
pub mod htlc_harness;
pub mod parity_client;

use crate::{
    ethereum_wallet::transaction::UnsignedTransaction,
    htlc_harness::{erc20_harness, Erc20HarnessParams, HTLC_TIMEOUT, SECRET},
};
use ethereum_support::{Bytes, H256, U256};
use spectral::prelude::*;
use testcontainers::clients::Cli;

#[test]
fn given_erc20_token_should_deploy_erc20_htlc_and_fund_htlc() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) =
        erc20_harness(&docker, Erc20HarnessParams::default());

    assert_eq!(client.token_balance_of(token, htlc_address), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(1000));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    // check htlc funding
    assert_eq!(
        client.token_balance_of(token, htlc_address),
        U256::from(400)
    );
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));

    // Send correct secret to contract
    client.send_data(htlc_address, Some(Bytes(SECRET.to_vec())));

    assert_eq!(client.token_balance_of(token, htlc_address), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));
    assert_eq!(client.token_balance_of(token, bob), U256::from(400));
}

#[test]
fn given_funded_erc20_htlc_when_redeemed_with_secret_then_tokens_are_transferred() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) =
        erc20_harness(&docker, Erc20HarnessParams::default());

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    assert_eq!(
        client.token_balance_of(token, htlc_address),
        U256::from(400)
    );
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));

    // Send correct secret to contract
    client.send_data(htlc_address, Some(Bytes(SECRET.to_vec())));

    assert_eq!(client.token_balance_of(token, htlc_address), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));
    assert_eq!(client.token_balance_of(token, bob), U256::from(400));
}

#[test]
fn given_deployed_erc20_htlc_when_refunded_after_timeout_then_tokens_are_refunded() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) =
        erc20_harness(&docker, Erc20HarnessParams::default());

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    assert_eq!(
        client.token_balance_of(token, htlc_address),
        U256::from(400)
    );
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));

    // Wait for the contract to expire
    ::std::thread::sleep(HTLC_TIMEOUT);
    ::std::thread::sleep(HTLC_TIMEOUT);
    client.send_data(htlc_address, None);

    assert_eq!(client.token_balance_of(token, htlc_address), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(1000));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));
}

#[test]
fn given_deployed_erc20_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) =
        erc20_harness(&docker, Erc20HarnessParams::default());

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    assert_eq!(
        client.token_balance_of(token, htlc_address),
        U256::from(400)
    );
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));

    // Don't wait for the timeout and don't send a secret
    client.send_data(htlc_address, None);

    assert_eq!(
        client.token_balance_of(token, htlc_address),
        U256::from(400)
    );
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(600));
}

#[test]
fn given_not_enough_tokens_when_redeemed_token_balances_dont_change() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) = erc20_harness(
        &docker,
        Erc20HarnessParams {
            alice_initial_tokens: U256::from(200),
            ..Default::default()
        },
    );

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    assert_eq!(client.token_balance_of(token, htlc_address), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(200));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));

    // Send correct secret to contract
    client.send_data(htlc_address, Some(Bytes(SECRET.to_vec())));

    assert_eq!(client.token_balance_of(token, htlc_address), U256::from(0));
    assert_eq!(client.token_balance_of(token, alice), U256::from(200));
    assert_eq!(client.token_balance_of(token, bob), U256::from(0));
    assert_eq!(client.get_contract_code(htlc_address), Bytes::default());
}

#[test]
fn given_htlc_and_redeem_should_emit_redeem_log_msg() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) =
        erc20_harness(&docker, Erc20HarnessParams::default());

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    // Send correct secret to contract
    let transaction_receipt = client.send_data(htlc_address, Some(Bytes(SECRET.to_vec())));

    assert_that(&transaction_receipt.logs.len()).is_equal_to(2); // should contain token transfer logs (in 2 transactions) and redeemed log (in 1
                                                                 // transaction)
    let redeem_topic: H256 =
        "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413".into(); // keccak256(Redeemed())
    let refund_topic: H256 =
        "0x5D26862916391BF49478B2F5103B0720A842B45EF145A268F2CD1FB2AED55178".into(); // keccak256(Refunded())

    let topics: Vec<H256> = transaction_receipt
        .logs
        .into_iter()
        .flat_map(|s| s.topics)
        .collect();
    assert_that(&topics).has_length(4); // should contain token transfers log and redeemed log
    assert_that(&topics).contains(redeem_topic);
    assert_that(&topics).does_not_contain(refund_topic);
}

#[test]
fn given_htlc_and_refund_should_emit_refund_log_msg() {
    let docker = Cli::default();
    let (alice, bob, htlc_address, htlc, token, client, _handle, _container) =
        erc20_harness(&docker, Erc20HarnessParams::default());

    // fund erc20 htlc
    client.sign_and_send(|nonce, gas_price| UnsignedTransaction {
        nonce,
        gas_price,
        gas_limit: U256::from(100_000),
        to: Some(token),
        value: U256::from(0),
        data: Some(htlc.funding_tx_payload(htlc_address)),
    });

    // Wait for the contract to expire
    ::std::thread::sleep(HTLC_TIMEOUT);
    ::std::thread::sleep(HTLC_TIMEOUT);
    // Send correct secret to contract
    let transaction_receipt = client.send_data(htlc_address, None);

    assert_that(&transaction_receipt.logs.len()).is_equal_to(2); // should contain token transfer logs (in 2 transactions) and redeemed log (in 1
                                                                 // transaction)
    let redeem_topic: H256 =
        "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413".into(); // keccak256(Redeemed())
    let refund_topic: H256 =
        "0x5D26862916391BF49478B2F5103B0720A842B45EF145A268F2CD1FB2AED55178".into(); // keccak256(Refunded())

    let topics: Vec<H256> = transaction_receipt
        .logs
        .into_iter()
        .flat_map(|s| s.topics)
        .collect();
    assert_that(&topics).has_length(4); // should contain token transfers log and redeemed log
    assert_that(&topics).does_not_contain(redeem_topic);
    assert_that(&topics).contains(refund_topic);
}
