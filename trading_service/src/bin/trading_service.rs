#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin;
extern crate bitcoin_htlc;
extern crate bitcoin_rpc;
extern crate env_logger;
extern crate log;
extern crate rocket;
extern crate trading_service;

use std::env::var;
use trading_service::exchange_api_client::ExchangeApiUrl;
use trading_service::rocket_factory::create_rocket_instance;

fn main() {
    let _ = env_logger::init();
    let exchange_api_url = ExchangeApiUrl(var("EXCHANGE_SERVICE_URL").unwrap());

    let network = match var("BTC_NETWORK") {
        Ok(value) => match value.as_str() {
            "BTC_MAINNET" => bitcoin::network::constants::Network::Bitcoin,
            "BTC_TESTNET" => bitcoin::network::constants::Network::Testnet,
            "BTCORE_REGTEST" => bitcoin::network::constants::Network::BitcoinCoreRegtest,
            _ => panic!(
                "Please set environment variable BTC_NETWORK to one of the following values:\n\
                 - BTC_MAINNET\n- BTC_TESTNET\n- BTCORE_REGTEST"
            ),
        },
        Err(_) => bitcoin::network::constants::Network::BitcoinCoreRegtest,
    };

    create_rocket_instance(exchange_api_url, network).launch();
}
