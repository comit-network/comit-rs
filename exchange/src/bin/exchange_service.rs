#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate exchange;
extern crate log;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

use exchange::rocket_factory::create_rocket_instance;
use exchange::types::Offers;
use exchange::types::TreasuryApiUrl;
use std::env::var;

fn main() {
    let treasury_api_url = TreasuryApiUrl(var("TREASURY_SERVICE_URL").unwrap());
    let offers = Offers::new();
    create_rocket_instance(treasury_api_url, offers).launch();
}
