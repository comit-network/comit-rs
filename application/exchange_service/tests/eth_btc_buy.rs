#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate exchange_service;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate serde_json;

use bitcoin_support::Network;
use ethereum_support::{web3, Bytes, H256};
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::InMemoryEventStore;
use exchange_service::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    ethereum_service::{self, BlockingEthereumApi},
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    treasury_api_client::FakeApiClient,
};
use rocket::{
    http::{ContentType, Status},
    local::{Client, LocalResponse},
};
use serde::Deserialize;
use std::{str::FromStr, sync::Arc};

fn request_offer(client: &mut Client) -> LocalResponse {
    let request = client
        .post("/trades/ETH-BTC/buy-offers")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 42 }"#);
    request.dispatch()
}

fn request_order<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
    let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "contract_secret_lock": "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec",
                    "client_refund_address": "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
                    "client_success_address": "0x956abb53d3ccbf24cf2f8c6e334a56d4b6c50440",
                    "client_contract_time_lock": 24
                  }"#,
            );
    request.dispatch()
}

fn notify_about_funding<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
    let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-order-htlc-funded", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "transaction_id": "a02e9dc0ddc3d8200cc4be0e40a1573519a1a1e9b15e0c4c296fcaa65da80d43",
                    "vout" : 0
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

struct StaticEthereumApi;

impl BlockingEthereumApi for StaticEthereumApi {
    fn send_raw_transaction(&self, _rlp: Bytes) -> Result<H256, web3::Error> {
        Ok(H256::new())
    }
}

fn create_rocket_client() -> Client {
    let rocket = create_rocket_instance(
        Arc::new(FakeApiClient),
        InMemoryEventStore::new(),
        Arc::new(ethereum_service::EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        Arc::new(bitcoin_rpc_client::BitcoinStubClient::new()),
        "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
        bitcoin_support::PrivateKey::from_str(
            "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
        ).unwrap()
            .secret_key()
            .clone()
            .into(),
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap(),
        Network::Regtest,
        Arc::new(StaticBitcoinFeeService::new(50.0)),
    );
    rocket::local::Client::new(rocket).unwrap()
}

#[test]
fn given_an_offer_request_then_return_valid_offer_response() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let mut response = request_offer(&mut client);
    assert_eq!(response.status(), Status::Ok);

    let offer_response = response.body_json::<serde_json::Value>();

    assert_eq!(
        offer_response["symbol"], "ETH-BTC",
        "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
        offer_response
    );
}

#[test]
fn given_a_trade_request_when_buy_offer_was_done_then_return_valid_trade_response() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let uid = {
        let mut response = request_offer(&mut client);
        assert_eq!(response.status(), Status::Ok);

        let offer_response = response.body_json::<serde_json::Value>();

        assert_eq!(
            offer_response["symbol"], "ETH-BTC",
            "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
            offer_response
        );

        offer_response["uid"].as_str().unwrap().to_string()
    };

    {
        let mut response = request_order(&mut client, &uid);
        assert_eq!(response.status(), Status::Ok);

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Response {
            exchange_contract_time_lock: i64,
        }

        serde_json::from_str::<Response>(&response.body_string().unwrap()).unwrap();
    }
}

#[test]
fn given_a_order_request_without_offer_should_fail() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let uid = "d9ee2df7-c330-4893-8345-6ba171f96e8f";

    {
        let response = request_order(&mut client, uid);
        assert_eq!(response.status(), Status::BadRequest);
    }
}

#[test]
fn given_two_orders_request_with_same_uid_should_fail() {
    let _ = env_logger::try_init();

    let mut client = create_rocket_client();

    let uid = {
        let mut response = request_offer(&mut client);
        assert_eq!(response.status(), Status::Ok);

        let response =
            serde_json::from_str::<serde_json::Value>(&response.body_string().unwrap()).unwrap();

        response["uid"].as_str().unwrap().to_string()
    };

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

    let trade_id = {
        let mut response = request_offer(&mut client);

        assert_eq!(response.status(), Status::Ok);
        response.body_json::<serde_json::Value>()["uid"]
            .as_str()
            .unwrap()
            .to_string()
    };

    {
        let response = request_order(&mut client, &trade_id);
        assert_eq!(response.status(), Status::Ok)
    }

    {
        let response = notify_about_funding(&mut client, &trade_id);
        assert_eq!(response.status(), Status::Ok)
    }
}
