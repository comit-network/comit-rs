#![warn(
    unused_results,
    unused_extern_crates,
    missing_debug_implementations
)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate ledger_query_service;
extern crate pretty_env_logger;
extern crate rocket;
#[macro_use]
extern crate log;

use ledger_query_service::{
    BitcoindZmqListener, DefaultTransactionProcessor, EthereumWeb3BlockPoller,
    InMemoryQueryRepository, InMemoryQueryResultRepository, LinkFactory,
};
use std::{env::var, sync::Arc, thread, time::Duration};

fn main() {
    let _ = pretty_env_logger::try_init();

    let config = rocket::Config::development().unwrap();

    // TODO: Read that stuff from the environment
    let link_factory = LinkFactory::new("http", "localhost", Some(config.port));

    let server_builder =
        ledger_query_service::server_builder::ServerBuilder::create(config, link_factory);

    let server_builder = match var("BITCOIN_ZMQ_ENDPOINT") {
        Err(_) => server_builder,
        Ok(zmq_endpoint) => {
            //e.g. tcp://127.0.0.1:28332
            info!("Starting BitcoinZmqListener on {}", zmq_endpoint);

            let query_repository = Arc::new(InMemoryQueryRepository::default());
            let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

            let bitcoin_transaction_processor = DefaultTransactionProcessor::new(
                query_repository.clone(),
                query_result_repository.clone(),
            );

            let _ = thread::spawn(move || {
                let bitcoind_zmq_listener =
                    BitcoindZmqListener::new(zmq_endpoint.as_str(), bitcoin_transaction_processor);

                match bitcoind_zmq_listener {
                    Ok(mut listener) => listener.start(),
                    Err(e) => error!("Failed to start BitcoinZmqListener! {:?}", e),
                }
            });
            server_builder.register_bitcoin(query_repository, query_result_repository)
        }
    };

    let server_builder = match var("ETHEREUM_WEB3_ENDPOINT") {
        Err(_) => server_builder,
        Ok(web3_endpoint) => {
            info!("Starting EthereumSimpleListener on {}", web3_endpoint);

            let polling_wait_time = match var("ETHEREUM_POLLING_TIME_SEC") {
                Err(_) => 17,
                Ok(var) => var.parse().unwrap(),
            };
            let polling_wait_time = Duration::from_secs(polling_wait_time);

            let query_repository = Arc::new(InMemoryQueryRepository::default());
            let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

            let ethereum_transaction_processor = DefaultTransactionProcessor::new(
                query_repository.clone(),
                query_result_repository.clone(),
            );

            let _ = thread::spawn(move || {
                let ethereum_web3_poller = EthereumWeb3BlockPoller::new(
                    web3_endpoint.as_str(),
                    polling_wait_time,
                    ethereum_transaction_processor,
                );

                match ethereum_web3_poller {
                    Ok(listener) => listener.start(),
                    Err(e) => error!("Failed to start EthereumSimpleListener! {:?}", e),
                }
            });
            server_builder.register_ethereum(query_repository, query_result_repository)
        }
    };

    let _ = server_builder.build().launch();
}
