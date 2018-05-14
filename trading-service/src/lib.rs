#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate rocket_codegen;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

mod exchange_api_client;

pub mod types;
pub mod routes;
pub mod rocket_factory;