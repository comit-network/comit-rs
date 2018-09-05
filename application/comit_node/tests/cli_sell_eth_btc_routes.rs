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
extern crate ethereum_wallet;
extern crate serde_json;
extern crate uuid;

mod common;
mod mocks;

use bitcoin_rpc_client::TransactionId;
use bitcoin_support::Network;
use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService, bitcoin_service::BitcoinService,
    comit_node_api_client::FakeApiClient as FakeComitNodeApiClient, ethereum_service,
    gas_price_service::StaticGasPriceService, rocket_factory::create_rocket_instance,
};
use common::OfferResponseBody;
use ethereum_wallet::fake::StaticFakeWallet;
use event_store::InMemoryEventStore;
use mocks::{BitcoinRpcClientMock, StaticEthereumApi};
use rocket::{
    http::{ContentType, Status},
    local::Client,
};
use std::{str::FromStr, sync::Arc};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    address_to_fund: String,
    btc_amount: String,
    eth_amount: String,
    data: String,
    gas: u64,
}

impl PartialEq for RequestToFund {
    fn eq(&self, other: &RequestToFund) -> bool {
        self.address_to_fund == other.address_to_fund
            && self.btc_amount == other.btc_amount
            && self.eth_amount == other.eth_amount
            && self.gas == other.gas
            && self.data.len() > 0
            && other.data.len() > 0
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
        Network::Testnet,
        Arc::new(api_client),
    );
    rocket::local::Client::new(rocket).unwrap()
}

#[test]
fn post_sell_offer_of_x_eth_for_btc() {
    let client = create_rocket_client();

    let request = client
        .post("/cli/trades/ETH-BTC/sell-offers")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 42 }"#);

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);
    let offer_response =
        serde_json::from_str::<OfferResponseBody>(&response.body_string().unwrap()).unwrap();

    assert_eq!(
        offer_response,
        OfferResponseBody {
            uid: String::from(""),
            symbol: String::from("ETH-BTC"),
            rate: 0.1,
            buy_amount: String::from("420000000"),
            sell_amount: String::from("42000000000000000000"),
        },
        "offer_response has correct fields"
    );
}

#[test]
fn post_sell_order_of_x_eth_for_btc() {
    let client = create_rocket_client();

    let request = client
        .post("/cli/trades/ETH-BTC/sell-offers")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 42 }"#);

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);
    let offer_response =
        serde_json::from_str::<OfferResponseBody>(&response.body_string().unwrap()).unwrap();
    let uid = offer_response.uid;

    let request = client
        .post(format!("/cli/trades/ETH-BTC/{}/sell-orders", uid))
        .header(ContentType::JSON)
        .body(r#"{ "alice_success_address": "tb1qj3z3ymhfawvdp4rphamc7777xargzufztd44fv", "alice_refund_address" : "0x4a965b089f8cb5c75efaa0fbce27ceaaf7722238" }"#);

    let mut response = request.dispatch();
    assert_eq!(response.status(), Status::Ok);
    let request_to_fund =
        serde_json::from_str::<RequestToFund>(&response.body_string().unwrap()).unwrap();

    assert_eq!(
        request_to_fund,
        RequestToFund {
            address_to_fund: String::from("0x0000000000000000000000000000000000000000"),
            btc_amount: String::from("420000000"),
            eth_amount: String::from("42000000000000000000"),
            data: String::from("some random data for passing the partial equal"),
            gas: 21_000u64,
        },
        "request_to_fund has correct address_to_fund"
    );
}
