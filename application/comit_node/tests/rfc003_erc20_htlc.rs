#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
extern crate comit_node;
extern crate ethereum_support;
extern crate hex;
extern crate pretty_env_logger;
extern crate secp256k1_support;
#[macro_use]
extern crate log;
extern crate tc_web3_client;
extern crate testcontainers;
#[macro_use]
extern crate lazy_static;
extern crate rlp;
extern crate tiny_keccak;

mod ethereum_wallet;
mod htlc_harness;
mod parity_client;

use ethereum_support::{Bytes, U256};
use ethereum_wallet::transaction::UnsignedTransaction;
use htlc_harness::{erc20_harness, Erc20HarnessParams, HTLC_TIMEOUT, SECRET};
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
