#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_support;
extern crate ledger_query_service;
extern crate rocket;
#[macro_use]
extern crate log;
use ledger_query_service::{
    BitcoinZmqListener, DefaultTransactionProcessor, InMemoryQueryRepository,
    InMemoryQueryResultRepository, LinkFactory,
};
use std::sync::Arc;

fn main() {
    let config = rocket::Config::development().unwrap();

    // TODO: Read that stuff from the environment
    let link_factory = LinkFactory::new("http", "localhost", Some(config.port));
    let query_repository = Arc::new(InMemoryQueryRepository::default());
    let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let bitcoin_transaction_processor =
        DefaultTransactionProcessor::new(query_repository.clone(), query_result_repository.clone());

    ::std::thread::spawn(move || {
        let mut bitcoin_zmq_listener =
            BitcoinZmqListener::new("tcp://127.0.0.1:28332", bitcoin_transaction_processor);
        bitcoin_zmq_listener.start();
    });

    ledger_query_service::server::create(
        config,
        link_factory,
        query_repository,
        query_result_repository,
    ).launch();
}
