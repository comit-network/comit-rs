#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate rocket;
extern crate trading_service;

use std::env::var;
use trading_service::exchange_api_client::ExchangeApiUrl;
use trading_service::rocket_factory::create_rocket_instance;

fn main() {
    let exchange_api_url = ExchangeApiUrl(var("EXCHANGE_SERVICE_URL").unwrap());
    create_rocket_instance(exchange_api_url).launch();
}
