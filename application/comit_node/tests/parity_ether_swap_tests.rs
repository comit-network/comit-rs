extern crate bitcoin_htlc;
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate common_types;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate hex;
extern crate pretty_env_logger;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate tc_parity_parity;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde_json;
extern crate spectral;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;
#[macro_use]
extern crate lazy_static;
extern crate web3;

mod htlc_harness;
mod parity_client;

use ethereum_support::{Bytes, EthereumQuantity, U256};
use htlc_harness::*;
use spectral::prelude::*;
use std::time::Duration;

const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";
const HTLC_TIMEOUT: Duration = Duration::from_secs(5);
const HTLC_GAS_COST: u64 = 8879000;

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let (alice, bob, htlc, _, client, _handle, _container) = harness(TestHarnessParams {
        alice_initial_ether: EthereumQuantity::from_eth(1.0),
        htlc_type: HtlcType::Eth {
            htlc_eth_value: EthereumQuantity::from_eth(0.4),
        },
        htlc_timeout: HTLC_TIMEOUT,
        htlc_secret: SECRET.clone(),
    });

    let htlc = assert_that(&htlc).is_ok().subject.clone();

    assert_eq!(
        client.eth_balance_of(bob),
        EthereumQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EthereumQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EthereumQuantity::from_eth(0.4).wei()
    );

    // Send correct secret to contract
    let _ = client.send_data(htlc, Some(Bytes(SECRET.to_vec())));

    assert_eq!(
        client.eth_balance_of(bob),
        EthereumQuantity::from_eth(0.4).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EthereumQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EthereumQuantity::from_eth(0.0).wei()
    );
}

#[test]
fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {
    let (alice, bob, htlc, _, client, _handle, _container) = harness(TestHarnessParams {
        alice_initial_ether: EthereumQuantity::from_eth(1.0),
        htlc_type: HtlcType::Eth {
            htlc_eth_value: EthereumQuantity::from_eth(0.4),
        },
        htlc_timeout: HTLC_TIMEOUT,
        htlc_secret: SECRET.clone(),
    });

    let htlc = assert_that(&htlc).is_ok().subject.clone();

    assert_eq!(
        client.eth_balance_of(bob),
        EthereumQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EthereumQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EthereumQuantity::from_eth(0.4).wei()
    );

    // Wait for the contract to expire
    ::std::thread::sleep(HTLC_TIMEOUT);
    ::std::thread::sleep(HTLC_TIMEOUT);
    let _ = client.send_data(htlc, None);

    assert_eq!(
        client.eth_balance_of(bob),
        EthereumQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EthereumQuantity::from_eth(1.0).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EthereumQuantity::from_eth(0.0).wei()
    );
}

#[test]
fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let (alice, bob, htlc, _, client, _handle, _container) = harness(TestHarnessParams {
        alice_initial_ether: EthereumQuantity::from_eth(1.0),
        htlc_type: HtlcType::Eth {
            htlc_eth_value: EthereumQuantity::from_eth(0.4),
        },
        htlc_timeout: HTLC_TIMEOUT,
        htlc_secret: SECRET.clone(),
    });

    let htlc = assert_that(&htlc).is_ok().subject.clone();

    assert_eq!(
        client.eth_balance_of(bob),
        EthereumQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EthereumQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EthereumQuantity::from_eth(0.4).wei()
    );

    // Wait for the contract to expire
    let _ = client.send_data(htlc, None);

    assert_eq!(
        client.eth_balance_of(bob),
        EthereumQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EthereumQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EthereumQuantity::from_eth(0.4).wei()
    );
}
