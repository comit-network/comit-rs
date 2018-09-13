extern crate bitcoin_htlc;
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate ganache_rust_web3;
extern crate hex;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_derive;
extern crate tc_parity_parity;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde_json;
extern crate spectral;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;
#[macro_use]
extern crate lazy_static;
extern crate web3;

mod parity_client;

use comit_node::{
    gas_price_service::StaticGasPriceService,
    swap_protocols::rfc003::ledger_htlc_service::{
        Erc20HtlcParams, EthereumService, LedgerHtlcService,
    },
};
use common_types::{seconds::Seconds, secret::Secret};
use ethereum_support::{web3::transports::EventLoopHandle, Address, *};
use ethereum_wallet::fake::StaticFakeWallet;
use parity_client::ParityClient;
use secp256k1_support::KeyPair;
use spectral::prelude::*;
use std::{sync::Arc, time::Duration};
use tc_parity_parity::ParityEthereum;
use testcontainers::{clients::DockerCli, Container, Docker};
use web3::types::Bytes;

const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";
const HTLC_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn given_deployed_erc20_htlc_when_redeemed_with_secret_then_tokens_are_transferred() {
    let (alice, bob, htlc, token_contract, client, _handle, _container) = arrange();

    assert_eq!(client.balance_of(token_contract, bob), U256::from(0));
    assert_eq!(client.balance_of(token_contract, alice), U256::from(600));
    assert_eq!(client.balance_of(token_contract, htlc), U256::from(400));

    // Send correct secret to contract
    let _ = client.send_data(htlc, Some(Bytes(SECRET.to_vec())));

    assert_eq!(client.balance_of(token_contract, bob), U256::from(400));
    assert_eq!(client.balance_of(token_contract, alice), U256::from(600));
    assert_eq!(client.balance_of(token_contract, htlc), U256::from(0));
}

#[test]
fn given_deployed_erc20_htlc_when_refunded_after_timeout_then_tokens_are_refunded() {
    let (alice, bob, htlc, token_contract, client, _handle, _container) = arrange();

    assert_eq!(client.balance_of(token_contract, bob), U256::from(0));
    assert_eq!(client.balance_of(token_contract, alice), U256::from(600));
    assert_eq!(client.balance_of(token_contract, htlc), U256::from(400));

    // Wait for the contract to expire
    ::std::thread::sleep(HTLC_TIMEOUT);
    ::std::thread::sleep(HTLC_TIMEOUT);
    let _ = client.send_data(htlc, None);

    assert_eq!(client.balance_of(token_contract, bob), U256::from(0));
    assert_eq!(client.balance_of(token_contract, alice), U256::from(1000));
    assert_eq!(client.balance_of(token_contract, htlc), U256::from(0));
}

#[test]
fn given_deployed_erc20_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let (alice, bob, htlc, token_contract, client, _handle, _container) = arrange();

    assert_eq!(client.balance_of(token_contract, bob), U256::from(0));
    assert_eq!(client.balance_of(token_contract, alice), U256::from(600));
    assert_eq!(client.balance_of(token_contract, htlc), U256::from(400));

    // Don't wait for the timeout and don't send a secret
    let _ = client.send_data(htlc, None);

    assert_eq!(client.balance_of(token_contract, bob), U256::from(0));
    assert_eq!(client.balance_of(token_contract, alice), U256::from(600));
    assert_eq!(client.balance_of(token_contract, htlc), U256::from(400));
}

fn arrange() -> (
    Address,
    Address,
    Address,
    Address,
    ParityClient,
    EventLoopHandle,
    Container<DockerCli, ParityEthereum>,
) {
    let _ = env_logger::try_init();

    let (alice_keypair, alice) =
        new_account("63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992");
    let (_, bob) = new_account("f8218ebf6e2626bd1415c18321496e0c5725f0e1d774c7c2eab69f7650ad6e82");

    let container = DockerCli::new().run(ParityEthereum::default());
    let (event_loop, web3) = tc_web3_client::new(&container);

    let client = ParityClient::new(web3);
    client.give_eth_to(alice, EthereumQuantity::from_eth(1.0));

    let token_contract = client.deploy_token_contract();
    client.mint_1000_tokens(token_contract, alice);

    let ethereum_service = EthereumService::new(
        Arc::new(StaticFakeWallet::from_key_pair(alice_keypair.clone())),
        Arc::new(StaticGasPriceService::default()),
        Arc::new(tc_web3_client::new(&container)),
        0,
    );

    let htlc_params = Erc20HtlcParams {
        refund_address: alice,
        success_address: bob,
        time_lock: Seconds::from(HTLC_TIMEOUT),
        amount: U256::from(400),
        secret_hash: Secret::from(SECRET.clone()).hash(),
        token_contract_address: token_contract,
    };

    let result = ethereum_service.deploy_htlc(htlc_params);
    let htlc_deployment_tx_id = assert_that(&result).is_ok().subject;

    let htlc = client.get_contract_address(htlc_deployment_tx_id.clone());

    (
        alice,
        bob,
        htlc,
        token_contract,
        client,
        event_loop,
        container,
    )
}

fn new_account(secret_key: &str) -> (KeyPair, Address) {
    let keypair = KeyPair::from_secret_key_hex(secret_key).unwrap();
    let address = keypair.public_key().to_ethereum_address();

    (keypair, address)
}
