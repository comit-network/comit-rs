#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_support;
extern crate ledger_query_service;
extern crate pretty_env_logger;
extern crate rocket;
#[macro_use]
extern crate log;
extern crate futures;
extern crate web3;

use ledger_query_service::{
    bitcoin_query::BitcoinQuery, ethereum_query::EthereumQuery, BitcoindZmqListener,
    DefaultTransactionProcessor, EthereumWeb3BlockPoller, InMemoryQueryRepository,
    InMemoryQueryResultRepository, LinkFactory, QueryRepository, QueryResultRepository,
};
use std::{env::var, sync::Arc, time::Duration};

fn main() {
    let _ = pretty_env_logger::try_init();

    let config = rocket::Config::development().unwrap();

    // TODO: Read that stuff from the environment
    let link_factory = LinkFactory::new("http", "localhost", Some(config.port));
    let mut bitcoin_repositories: Option<(
        Arc<QueryRepository<BitcoinQuery>>,
        Arc<QueryResultRepository<BitcoinQuery>>,
    )> = None;
    let mut ethereum_repositories: Option<(
        Arc<QueryRepository<EthereumQuery>>,
        Arc<QueryResultRepository<EthereumQuery>>,
    )> = None;

    if let Ok(zmq_endpoint) = var("BITCOIN_ZMQ_ENDPOINT") {
        //e.g. tcp://127.0.0.1:28332
        info!("Starting BitcoinZmqListener on {}", zmq_endpoint);

        let query_repository = Arc::new(InMemoryQueryRepository::default());
        let query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

        let bitcoin_transaction_processor = DefaultTransactionProcessor::new(
            query_repository.clone(),
            query_result_repository.clone(),
        );

        bitcoin_repositories = Some((query_repository, query_result_repository));

        ::std::thread::spawn(move || {
            let bitcoind_zmq_listener =
                BitcoindZmqListener::new(zmq_endpoint.as_str(), bitcoin_transaction_processor);

            match bitcoind_zmq_listener {
                Ok(mut listener) => listener.start(),
                Err(e) => error!("Failed to start BitcoinZmqListener! {:?}", e),
            }
        });
    }

    if let Ok(web3_endpoint) = var("ETHEREUM_WEB3_ENDPOINT") {
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

        ethereum_repositories = Some((query_repository, query_result_repository));

        ::std::thread::spawn(move || {
            let ethereum_simple_listener = EthereumWeb3BlockPoller::new(
                web3_endpoint.as_str(),
                polling_wait_time,
                ethereum_transaction_processor,
            );

            match ethereum_simple_listener {
                Ok(listener) => listener.start(),
                Err(e) => error!("Failed to start EthereumSimpleListener! {:?}", e),
            }
        });
    }

    ledger_query_service::server::create(
        config,
        link_factory,
        bitcoin_repositories,
        ethereum_repositories,
    ).launch();
}
