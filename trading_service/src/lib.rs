#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
extern crate lazy_static;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

extern crate crypto;
extern crate rand;

#[macro_use]
extern crate log;
extern crate serde;

pub mod exchange_api_client;

pub mod offer;
pub mod rocket_factory;
pub mod routes;
pub mod secret;
pub mod symbol;
