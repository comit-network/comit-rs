#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate trading_service;

use std::env::var;
use trading_service::rocket_factory::create_rocket_instance;
use trading_service::types::{ExchangeApiUrl, Offers};

fn main() {
    let exchange_api_url = ExchangeApiUrl(var("EXCHANGE_SERVICE_URL").unwrap());
    let offers = Offers::new();
    create_rocket_instance(exchange_api_url, offers).launch();
}
