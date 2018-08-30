#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate event_store;
extern crate exchange_service;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate serde_json;

mod mocks;

use bitcoin_rpc_client::TransactionId;
use bitcoin_support::Network;
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::InMemoryEventStore;
use exchange_service::{
    bitcoin_fee_service::StaticBitcoinFeeService, bitcoin_service::BitcoinService,
    ethereum_service, gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance, treasury_api_client::FakeApiClient,
};
use mocks::{BitcoinRpcClientMock, StaticEthereumApi};
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

fn notify_about_revealed_secret<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
    let request = client
        .post(format!("/trades/ETH-BTC/{}/buy-order-secret-revealed", uid).to_string())
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
    let exchange_success_address =
        bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap();
    let bitcoin_service = Arc::new(BitcoinService::new(
        Arc::new(BitcoinRpcClientMock::new(
            TransactionId::from_str(
                "d54994ece1d11b19785c7248868696250ab195605b469632b7bd68130e880c9a",
            ).unwrap(),
        )),
        bitcoin_support::Network::Regtest,
        bitcoin_fee_service.clone(),
        exchange_success_address,
    ));

    let rocket = create_rocket_instance(
        Arc::new(FakeApiClient),
        InMemoryEventStore::new(),
        Arc::new(ethereum_service::EthereumService::new(
            Arc::new(StaticFakeWallet::account0()),
            Arc::new(StaticGasPriceService::default()),
            Arc::new(StaticEthereumApi),
            0,
        )),
        bitcoin_service,
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
        bitcoin_fee_service,
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
            exchange_contract_time_lock: u32,
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

#[test]
fn given_an_deployed_htlc_and_a_secret_should_redeem_secret() {
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

    {
        let response = notify_about_revealed_secret(&mut client, &trade_id);
        println!("{:?}", response);

        assert_eq!(response.status(), Status::Ok)
    }
}
