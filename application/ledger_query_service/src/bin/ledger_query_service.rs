#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate ledger_query_service;
extern crate rocket;
use ledger_query_service::{InMemoryQueryRepository, InMemoryQueryResultRepository, LinkFactory};
use std::sync::Arc;

fn main() {
    let config = rocket::Config::development().unwrap();

    // TODO: Read that stuff from the environment
    let link_factory = LinkFactory::new("http", "localhost", Some(config.port));
    let query_repository = InMemoryQueryRepository::default();
    let query_result_repository = InMemoryQueryResultRepository::default();

    ledger_query_service::server::create(
        config,
        link_factory,
        Arc::new(query_repository),
        Arc::new(query_result_repository),
    ).launch();
}
