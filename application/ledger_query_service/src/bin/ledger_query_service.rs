#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_support;
extern crate ledger_query_service;
extern crate pretty_env_logger;
extern crate rocket;
#[macro_use]
extern crate log;
use ledger_query_service::{
    BitcoindZmqListener, DefaultTransactionProcessor, InMemoryQueryRepository,
    InMemoryQueryResultRepository, LinkFactory,
};
use std::sync::Arc;

fn main() {
    let _ = pretty_env_logger::try_init();

    let config = rocket::Config::development().unwrap();

    // TODO: Read that stuff from the environment
    let link_factory = LinkFactory::new("http", "localhost", Some(config.port));
    let zmq_endpoint = "tcp://127.0.0.1:28332";

    let query_repository = Arc::new(InMemoryQueryRepository::default());
    let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let bitcoin_transaction_processor =
        DefaultTransactionProcessor::new(query_repository.clone(), query_result_repository.clone());

    ::std::thread::spawn(move || {
        let bitcoind_zmq_listener =
            BitcoindZmqListener::new(zmq_endpoint, bitcoin_transaction_processor);

        match bitcoind_zmq_listener {
            Ok(mut listener) => listener.start(),
            Err(e) => error!("Failed to start BitcoinZmqListener! {:?}", e),
        }
    });

    ledger_query_service::server::create(
        config,
        link_factory,
        query_repository,
        query_result_repository,
    ).launch();
}
