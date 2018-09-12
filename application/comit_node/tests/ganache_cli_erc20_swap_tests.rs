extern crate bitcoin_htlc;
extern crate bitcoin_support;
extern crate ethereum_support;
extern crate event_store;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_rpc_client;
extern crate comit_node;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_wallet;
extern crate ganache_rust_web3;
extern crate hex;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde_json;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;

mod common;
use comit_node::swap_protocols::rfc003::ethereum::Erc20Htlc;
use common::GanacheClient;
use common_types::secret::Secret;
use ethereum_support::*;
use std::time::Duration;

const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";
const ONE_HOUR: Duration = Duration::from_secs(60 * 60);

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let _ = env_logger::try_init();

    let alice: Address = "147ba99ef89c152f8004e91999fee87bda6cbc3e".into();
    let bob: Address = "96984c3e77f38ed01d1c3d98f4bd7c8b11d51d7e".into();
    //    let contract_owner: Address = "03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c".into();
    let htlc_contract: Address = "1e637bb1935f820390d746b241df4f6a0347884f".into();

    let secret = Secret::from(SECRET.clone());

    let mut client = GanacheClient::new();

    let contract = client.deploy_token_contract(alice);

    client.mint_1000_tokens(alice, contract, alice);
    client.approve_transfer(alice, contract, htlc_contract);

    let alice_balance = client.get_token_balance(contract, alice);
    assert_eq!(alice_balance, U256::from(1000));

    let htlc = Erc20Htlc::new(
        ONE_HOUR,
        alice,
        bob,
        secret.hash(),
        htlc_contract,
        contract,
        U256::from(400),
    );

    let gas_used = client.deploy(alice, htlc, 0);

    let contract_token_balance = client.get_token_balance(contract, htlc_contract);
    assert_eq!(contract_token_balance, U256::from(400));

    // redeem HTLC
    //
    //    let bob_balance_after = client.get_token_balance(contract, bob);
    //    assert_eq!(bob_balance_after, U256::from(500));
}
//
//#[test]
//fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {
//    let _ = env_logger::try_init();
//
//    let refund_address: Address = "03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c".into();
//    let success_address: Address = "25818640c330b071acf5fc836fe0b762a769523d".into();
//
//    let secret = Secret::from(SECRET.clone());
//
//    let htlc = Htlc::new(ONE_HOUR, refund_address, success_address, secret.hash());
//
//}
//
//#[test]
//fn given_advanced_timestamp_when_deployed_contract_cannot_yet_be_refunded() {
//    let _ = env_logger::try_init();
//
//    let refund_address: Address = "03744e31a6b9e6c6f604ff5d8ce1caef1c7bb58c".into();
//    let success_address: Address = "25818640c330b071acf5fc836fe0b762a769523d".into();
//
//    let secret = Secret::from(SECRET.clone());
//
//}
//
//#[test]
//fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
//    let _ = env_logger::try_init();
//
//    let refund_address: Address = "f507e0b7cb47e06bb725b605d463a56cef2c057d".into();
//    let success_address: Address = "70485b398676fa6c83fa600efd3e63a75e6ac5c2".into();
//
//    let secret = Secret::from(SECRET.clone());
//
//}
