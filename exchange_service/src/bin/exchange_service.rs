#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate exchange_service;
extern crate log;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

use exchange_service::event_store::EventStore;
use exchange_service::rocket_factory::create_rocket_instance;
use exchange_service::treasury_api_client::{DefaultApiClient, TreasuryApiUrl};
use std::env::var;
use std::sync::Arc;

fn main() {
    let treasury_api_url = TreasuryApiUrl(var("TREASURY_SERVICE_URL").unwrap());

    let api_client = DefaultApiClient {
        client: reqwest::Client::new(),
        url: treasury_api_url,
    };

    let event_store = EventStore::new();
    create_rocket_instance(Arc::new(api_client), event_store).launch();
}
