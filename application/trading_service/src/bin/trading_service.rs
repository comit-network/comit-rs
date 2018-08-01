#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_htlc;
extern crate bitcoin_rpc;
extern crate bitcoin_support;
extern crate env_logger;
extern crate event_store;
extern crate log;
extern crate logging;
extern crate rocket;
extern crate trading_service;

use bitcoin_support::Network;
use event_store::InMemoryEventStore;
use std::{env::var, sync::Arc};
use trading_service::{
    exchange_api_client::{DefaultApiClient, ExchangeApiUrl},
    rocket_factory::create_rocket_instance,
};

fn main() {
    logging::set_up_logging();
    let exchange_api_url = ExchangeApiUrl(var("EXCHANGE_SERVICE_URL").unwrap());

    let network = match var("BTC_NETWORK") {
        Ok(value) => match value.as_str() {
            "BTC_MAINNET" => Network::Bitcoin,
            "BTC_TESTNET" => Network::Testnet,
            "BTCORE_REGTEST" => Network::BitcoinCoreRegtest,
            _ => panic!(
                "Please set environment variable BTC_NETWORK to one of the following values:\n\
                 - BTC_MAINNET\n- BTC_TESTNET\n- BTCORE_REGTEST"
            ),
        },
        Err(_) => Network::BitcoinCoreRegtest,
    };

    create_rocket_instance(
        network,
        InMemoryEventStore::new(),
        Arc::new(DefaultApiClient::new(exchange_api_url)),
    ).launch();
}
