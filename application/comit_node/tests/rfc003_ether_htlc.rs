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

pub mod ethereum_wallet;
pub mod htlc_harness;
pub mod parity_client;

use crate::htlc_harness::{ether_harness, EtherHarnessParams, HTLC_TIMEOUT, SECRET};
use ethereum_support::{Bytes, EtherQuantity, U256};
use testcontainers::clients::Cli;

const HTLC_GAS_COST: u64 = 8879000;

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let docker = Cli::default();
    let (alice, bob, htlc, client, _handle, _container) =
        ether_harness(&docker, EtherHarnessParams::default());

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );

    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );

    // Send correct secret to contract
    client.send_data(htlc, Some(Bytes(SECRET.to_vec())));

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.4).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.0).wei()
    );
}

#[test]
fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {
    let docker = Cli::default();
    let (alice, bob, htlc, client, _handle, _container) =
        ether_harness(&docker, EtherHarnessParams::default());

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );

    // Wait for the contract to expire
    ::std::thread::sleep(HTLC_TIMEOUT);
    ::std::thread::sleep(HTLC_TIMEOUT);
    client.send_data(htlc, None);

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(1.0).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.0).wei()
    );
}

#[test]
fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let docker = Cli::default();
    let (alice, bob, htlc, client, _handle, _container) =
        ether_harness(&docker, EtherHarnessParams::default());

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );

    // Wait for the contract to expire
    client.send_data(htlc, None);

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );
}
