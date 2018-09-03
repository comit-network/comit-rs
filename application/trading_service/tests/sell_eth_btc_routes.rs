extern crate bitcoin_htlc;
extern crate bitcoin_support;
extern crate common_types;
extern crate ethereum_htlc;
extern crate ethereum_support;
extern crate event_store;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate trading_service;
extern crate uuid;

use bitcoin_support::{BitcoinQuantity, Network};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    TradingSymbol,
};
use ethereum_support::{Bytes, EthereumQuantity};
use event_store::InMemoryEventStore;
use rocket::{http::*, request::FromParam};
use std::{fmt, str::FromStr, sync::Arc};
use trading_service::{exchange_api_client::FakeApiClient, rocket_factory::create_rocket_instance};
use uuid::Uuid;

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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl From<Uuid> for TradeId {
    fn from(uuid: Uuid) -> Self {
        TradeId(uuid)
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl<'a> FromParam<'a> for TradeId {
    type Error = uuid::ParseError;

    fn from_param(param: &RawStr) -> Result<Self, <Self as FromParam>::Error> {
        Uuid::parse_str(param.as_str()).map(|uid| TradeId::from(uid))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody {
    pub uid: String,
    pub symbol: String,
    pub rate: f64,
    pub buy_amount: String,
    pub sell_amount: String,
}

impl PartialEq for OfferResponseBody {
    fn eq(&self, other: &OfferResponseBody) -> bool {
        self.symbol == other.symbol
            && self.rate == other.rate
            && self.buy_amount == other.buy_amount
            && self.sell_amount == other.sell_amount
    }
}

#[test]
fn post_sell_offer_of_x_eth_for_btc() {
    let api_client = FakeApiClient::new();

    let rocket = create_rocket_instance(
        Network::Testnet,
        InMemoryEventStore::new(),
        Arc::new(api_client),
    );
    let client = rocket::local::Client::new(rocket).unwrap();

    let request = client
        .post("/trades/ETH-BTC/sell-offers")
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
    let api_client = FakeApiClient::new();

    let rocket = create_rocket_instance(
        Network::Testnet,
        InMemoryEventStore::new(),
        Arc::new(api_client),
    );
    let client = rocket::local::Client::new(rocket).unwrap();

    let request = client
        .post("/trades/ETH-BTC/sell-offers")
        .header(ContentType::JSON)
        .body(r#"{ "amount": 42 }"#);

    let mut response = request.dispatch();

    assert_eq!(response.status(), Status::Ok);
    let offer_response =
        serde_json::from_str::<OfferResponseBody>(&response.body_string().unwrap()).unwrap();
    let uid = offer_response.uid;

    let request = client
        .post(format!("/trades/ETH-BTC/{}/sell-orders", uid))
        .header(ContentType::JSON)
        .body(r#"{ "client_success_address": "tb1qj3z3ymhfawvdp4rphamc7777xargzufztd44fv", "client_refund_address" : "0x4a965b089f8cb5c75efaa0fbce27ceaaf7722238" }"#);

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
