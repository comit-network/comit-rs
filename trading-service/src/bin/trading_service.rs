#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate trading_service;
extern crate rocket;

use std::env::var;
use trading_service::types::ExchangeApiUrl;

fn main() {
    rocket::ignite()
        .mount("/", routes![trading_service::routes::offers::post])
        .manage(ExchangeApiUrl(var("EXCHANGE_SERVICE_URL").unwrap()))
        .launch();
}