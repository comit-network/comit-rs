#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

#[macro_use]
extern crate log;

pub mod event_store;
pub mod rocket_factory;
pub mod routes;
pub mod treasury_api_client;
