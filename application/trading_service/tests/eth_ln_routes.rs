extern crate bitcoin_support;
extern crate ethereum_support;
extern crate event_store;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate trading_service;

mod common;

use bitcoin_support::Network;
use common::OfferResponseBody;
use event_store::InMemoryEventStore;
use rocket::http::*;
use std::sync::Arc;
use trading_service::{exchange_api_client::FakeApiClient, rocket_factory::create_rocket_instance};

#[test]
fn post_buy_offers_should_call_create_offer_and_return_offer() {
    let api_client = FakeApiClient::new();

    let rocket = create_rocket_instance(
        Network::Testnet,
        InMemoryEventStore::new(),
        Arc::new(api_client),
    );
    let client = rocket::local::Client::new(rocket).unwrap();

    let request = client
        .post("/trades/ETH-LN/sell-offers")
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
            symbol: String::from("ETH-LN"),
            rate: 0.1,
            buy_amount: String::from("420000000"),
            sell_amount: String::from("42000000000000000000"),
        },
        "offer_response has correct fields"
    );
}
