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
extern crate hex;
extern crate reqwest;
extern crate serde_json;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;

use bitcoin_rpc_client::TransactionId;
use bitcoin_support::Network;

mod mocks;

use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    comit_node_api_client::FakeApiClient as FakeComitNodeApiClient,
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    swap_protocols::rfc003::ledger_htlc_service::{BitcoinService, EthereumService},
};
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::InMemoryEventStore;
use mocks::{
    BitcoinRpcClientMock, OfferResponseBody, RedeemDetails, RequestToFund, StaticEthereumApi,
};
use rocket::{
    http::{ContentType, Status},
    local::Client,
};
use std::{str::FromStr, sync::Arc};

fn create_rocket_client() -> Client {
    let bitcoin_fee_service = Arc::new(StaticBitcoinFeeService::new(50.0));
    let bob_success_address =
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap();
    let bitcoin_service = Arc::new(BitcoinService::new(
        Arc::new(BitcoinRpcClientMock::new(
            TransactionId::from_str(
                "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
            ).unwrap(),
        )),
        bitcoin_support::Network::Regtest,
        bitcoin_fee_service.clone(),
        bob_success_address,
    ));

    let api_client = FakeComitNodeApiClient::new();

    let rocket = create_rocket_instance(
        Arc::new(InMemoryEventStore::new()),
        Arc::new(EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        bitcoin_service,
        "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
        bitcoin_support::PrivateKey::from_str(
            "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
        ).unwrap()
        .secret_key()
        .clone()
        .into(),
        Network::Testnet,
        Arc::new(api_client),
    );
    rocket::local::Client::new(rocket).unwrap()
}

// Secret: 12345678901234567890123456789012
// Secret hash: 51a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c

// Sender address: bcrt1qryj6ya9vqpph8w65992nhk64cs890vfy0khsfg
// Sender pubkey: 020c04eb8cb87485501e30b656f37439ea7866d7c58b3c38161e5793b68e712356
// Sender pubkey hash: 1925a274ac004373bb5429553bdb55c40e57b124

// Recipient address: bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap
// Recipient pubkey: 0298e113cc06bc862ac205f2c0f27ee8c0de98d0716537bbf74e2ea6f38a84d5dc
// Recipient pubkey hash: c021f17be99c6adfbcba5d38ee0d292c0399d2f5

// htlc script: 63a82051a488e06e9c69c555b8ad5e2c4629bb3135b96accd1f23451af75e06d3aee9c8876a914c021f17be99c6adfbcba5d38ee0d292c0399d2f567028403b17576a9141925a274ac004373bb5429553bdb55c40e57b1246888ac
#[test]
fn happy_path_buy_x_eth_for_btc() {
    let client = create_rocket_client();

    let request = client
        .post("/cli/trades/ETH-BTC/buy-offers")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 43 }"#);

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);
    let offer_response =
        serde_json::from_str::<OfferResponseBody>(&response.body_string().unwrap()).unwrap();

    assert_eq!(
        offer_response.symbol, "ETH-BTC",
        "offer_response has correct symbol"
    );
    let uid = offer_response.uid;

    let request = client
        .post(format!("/cli/trades/ETH-BTC/{}/buy-orders", uid))
        .header(ContentType::JSON)
        // some random addresses I pulled off the internet
        .body(r#"{ "alice_success_address": "0x4a965b089f8cb5c75efaa0fbce27ceaaf7722238", "alice_refund_address" : "tb1qj3z3ymhfawvdp4rphamc7777xargzufztd44fv" }"#);

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);

    let funding_request =
        serde_json::from_str::<RequestToFund>(&response.body_string().unwrap()).unwrap();
    assert!(funding_request.address_to_fund.starts_with("tb1"));

    let request = client
        .post(format!(
            "/ledger/trades/ETH-BTC/{}/buy-order-contract-deployed",
            uid
        )).header(ContentType::JSON)
        .body(r#"{ "contract_address" : "0x00a329c0648769a73afac7f9381e08fb43dbea72" }"#);

    let response = request.dispatch();

    assert_eq!(
        response.status(),
        Status::Ok,
        "buy-order-contract-deployed call is successful"
    );

    let request = client.get(format!("/cli/trades/ETH-BTC/{}/redeem-orders", uid));

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);

    let _redeem_details =
        serde_json::from_str::<RedeemDetails>(&response.body_string().unwrap()).unwrap();
}

// sha256 of htlc script: e6877a670b46b9913bdaed47084f2db8983c2a22c473f0aea1fa5c2ebc4fd8d4
