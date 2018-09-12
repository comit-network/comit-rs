#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate ganache_rust_web3;
extern crate serde_json;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate hex;

mod common;

use bitcoin_rpc_client::TransactionId;
use bitcoin_support::Network;
use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService, bitcoin_service::BitcoinService,
    comit_node_api_client::FakeApiClient as FakeComitNodeApiClient, ethereum_service,
    gas_price_service::StaticGasPriceService, rocket_factory::create_rocket_instance,
};
use common::mocks;
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::InMemoryEventStore;
use mocks::{BitcoinRpcClientMock, StaticEthereumApi};
use rocket::{
    http::{ContentType, Status},
    local::{Client, LocalResponse},
};
use serde::Deserialize;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

fn request_order<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
    let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "contract_secret_lock": "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec",
                    "alice_refund_address": "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
                    "alice_success_address": "0x956abb53d3ccbf24cf2f8c6e334a56d4b6c50440",
                    "alice_contract_time_lock": 24,
                    "buy_amount" : "1000000",
                    "sell_amount" : "10000000"
                  }"#,
            );
    request.dispatch()
}

fn notify_about_funding<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
    let request = client
            .post(format!("/ledger/trades/ETH-BTC/{}/buy-order-htlc-funded", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "transaction_id": "a02e9dc0ddc3d8200cc4be0e40a1573519a1a1e9b15e0c4c296fcaa65da80d43",
                    "vout" : 0
                  }"#,
            );
    request.dispatch()
}

fn notify_about_revealed_secret<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
    let request = client
        .post(format!("/ledger/trades/ETH-BTC/{}/buy-order-secret-revealed", uid).to_string())
        .header(ContentType::JSON)
        .body(
            r#"{
                    "secret": "e8aafba2be13ee611059bc756878933bee789cc1aec7c35e23054a44d071c80b"
                  }"#,
        );
    request.dispatch()
}

trait DeserializeAsJson {
    fn body_json<T>(&mut self) -> T
    where
        for<'de> T: Deserialize<'de>;
}

impl<'r> DeserializeAsJson for LocalResponse<'r> {
    fn body_json<T>(&mut self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        let body = self.body().unwrap().into_inner();

        serde_json::from_reader(body).unwrap()
    }
}

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
        InMemoryEventStore::new(),
        Arc::new(ethereum_service::EthereumService::new(
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
        Network::Regtest,
        Arc::new(api_client),
    );
    rocket::local::Client::new(rocket).unwrap()
}

#[test]
fn given_a_trade_request_when_buy_offer_was_done_then_return_valid_trade_response() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let uid = Uuid::new_v4().to_string();

    {
        let mut response = request_order(&mut client, &uid);
        assert_eq!(response.status(), Status::Ok);

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Response {
            bob_contract_time_lock: u32,
        }

        serde_json::from_str::<Response>(&response.body_string().unwrap()).unwrap();
    }
}

#[test]
fn given_two_orders_request_with_same_uid_should_fail() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let uid = Uuid::new_v4().to_string();

    {
        let response = request_order(&mut client, &uid);
        assert_eq!(response.status(), Status::Ok);
    }

    {
        let response = request_order(&mut client, &uid);
        assert_eq!(response.status(), Status::BadRequest);
    }
}

#[test]
fn given_an_accepted_trade_when_provided_with_funding_tx_should_deploy_htlc() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let trade_id = Uuid::new_v4().to_string();

    {
        let response = request_order(&mut client, &trade_id);
        assert_eq!(response.status(), Status::Ok)
    }

    {
        let response = notify_about_funding(&mut client, &trade_id);
        assert_eq!(response.status(), Status::Ok)
    }
}

#[test]
fn given_an_deployed_htlc_and_a_secret_should_redeem_secret() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let trade_id = Uuid::new_v4().to_string();

    {
        let response = request_order(&mut client, &trade_id);
        assert_eq!(response.status(), Status::Ok)
    }

    {
        let response = notify_about_funding(&mut client, &trade_id);
        assert_eq!(response.status(), Status::Ok)
    }

    {
        let response = notify_about_revealed_secret(&mut client, &trade_id);

        assert_eq!(response.status(), Status::Ok)
    }
}
