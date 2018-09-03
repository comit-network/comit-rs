extern crate bitcoin_support;
extern crate common_types;
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

use bitcoin_support::Network;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    TradingSymbol,
};
use event_store::InMemoryEventStore;
use rocket::{http::*, request::FromParam};
use std::{fmt, sync::Arc};
use trading_service::{exchange_api_client::FakeApiClient, rocket_factory::create_rocket_instance};
use uuid::Uuid;

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
    let offer_response = serde_json::from_str::<OfferResponseBody<Bitcoin, Ethereum>>(
        &response.body_string().unwrap(),
    ).unwrap();

    assert_eq!(
        offer_response.symbol,
        TradingSymbol::ETH_LN,
        "offer_response has correct symbol"
    );
    assert_eq!(
        offer_response.sell_amount,
        ethereum_support::EthereumQuantity::from_eth(42.0),
        "offer_response has correct sell amount"
    );
    assert_eq!(
        offer_response.buy_amount,
        bitcoin_support::BitcoinQuantity::from_bitcoin(4.2),
        "offer_response has correct buy amount"
    );
    assert_eq!(offer_response.rate, 0.1, "offer_response has correct rate");
}
