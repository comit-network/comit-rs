extern crate bitcoin_support;
extern crate common_types;
extern crate event_store;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde_json;
extern crate trading_service;
extern crate uuid;

use bitcoin_support::Network;
use common_types::TradingSymbol;
use event_store::InMemoryEventStore;
use rocket::http::*;
use std::sync::Arc;
use trading_service::{
    exchange_api_client::{FakeApiClient, OfferResponseBody},
    rocket_factory::create_rocket_instance,
};

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
        .post("/trades/ETH-LN/sell-offer")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 42 }"#);

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);
    let offer_response =
        serde_json::from_str::<OfferResponseBody>(&response.body_string().unwrap()).unwrap();

    assert_eq!(
        offer_response.symbol,
        TradingSymbol::ETH_LN,
        "offer_response has correct symbol"
    );
    assert_eq!(
        offer_response.sell_amount, "24",
        "offer_response has correct sell amount"
    );
    assert_eq!(
        offer_response.buy_amount, "10.0",
        "offer_response has correct buy amount"
    );
    assert_eq!(offer_response.rate, 0.42, "offer_response has correct rate");
}
