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
    address_to_fund: ethereum_support::Address,
    btc_amount: BitcoinQuantity,
    eth_amount: EthereumQuantity,
    data: ethereum_htlc::ByteCode,
    gas: u64,
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
pub struct OfferResponseBody<Buy: Ledger, Sell: Ledger> {
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: Buy::Quantity,
    pub sell_amount: Sell::Quantity,
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
    let offer_response = serde_json::from_str::<OfferResponseBody<Bitcoin, Ethereum>>(
        &response.body_string().unwrap(),
    ).unwrap();

    assert_eq!(
        offer_response.symbol,
        TradingSymbol::ETH_BTC,
        "offer_response has correct symbol"
    );
    assert_eq!(
        offer_response.buy_amount,
        bitcoin_support::BitcoinQuantity::from_bitcoin(4.2),
        "offer_response has correct buy amount"
    );
    assert_eq!(
        offer_response.sell_amount,
        ethereum_support::EthereumQuantity::from_eth(42.0),
        "offer_response has correct sell amount"
    );
    assert_eq!(
        offer_response.rate, 0.1,
        "offer_response has correct sell amount"
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
    let offer_response = serde_json::from_str::<OfferResponseBody<Bitcoin, Ethereum>>(
        &response.body_string().unwrap(),
    ).unwrap();
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
        request_to_fund.address_to_fund,
        "0000000000000000000000000000000000000000".parse().unwrap(),
        "request_to_fund has correct address_to_fund"
    );

    assert_eq!(
        request_to_fund.btc_amount,
        BitcoinQuantity::from_str("4.2").unwrap(),
        "request_to_fund has correct btc_amount"
    );
    assert_eq!(
        request_to_fund.eth_amount,
        EthereumQuantity::from_str("42").unwrap(),
        "request_to_fund has correct eth_amount"
    );

    let bytes: Bytes = request_to_fund.data.into();
    assert!(!bytes.0.is_empty(), "request_to_fund has htlc data");

    assert_eq!(
        request_to_fund.gas, 21_000u64,
        "request_to_fund has correct gas"
    );
}
