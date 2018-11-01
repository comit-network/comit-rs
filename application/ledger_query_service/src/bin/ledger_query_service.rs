#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]

extern crate ledger_query_service;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate bitcoin_rpc_client;
extern crate warp;

use ledger_query_service::{
    bitcoin::{BitcoinBlockQuery, BitcoinTransactionQuery},
    ethereum::{EthereumBlockQuery, EthereumTransactionQuery},
    settings::Settings,
    BitcoindZmqListener, DefaultBlockProcessor, EthereumWeb3BlockPoller, InMemoryQueryRepository,
    InMemoryQueryResultRepository, LinkFactory, RouteFactory,
};
use std::{
    env::{var, VarError},
    sync::Arc,
    thread,
    time::Duration,
};
use warp::{filters::BoxedFilter, Filter, Reply};

fn main() {
    let _ = pretty_env_logger::try_init();

    let settings = load_settings();

    info!("Starting up with {:#?}", settings);

    let bitcoin_rpc_client = Arc::new(bitcoin_rpc_client::BitcoinCoreClient::new(
        settings.bitcoin.node_url.as_str(),
        settings.bitcoin.node_username.as_str(),
        settings.bitcoin.node_password.as_str(),
    ));

    // TODO: Read that stuff from the environment
    let link_factory = LinkFactory::new("http", "localhost", Some(8080));
    let route_factory = RouteFactory::new(link_factory);

    let bitcoin_endpoint = var("BITCOIN_ZMQ_ENDPOINT");
    let bitcoin_routes = create_bitcoin_routes(&route_factory, bitcoin_endpoint);

    let ethereum_endpoint = var("ETHEREUM_WEB3_ENDPOINT");
    let ethereum_routes = create_ethereum_routes(&route_factory, ethereum_endpoint);

    let routes = bitcoin_routes.or(ethereum_routes);
    warp::serve(routes).run(([127, 0, 0, 1], 8080));
}

fn create_bitcoin_routes(
    route_factory: &RouteFactory,
    endpoint: Result<String, VarError>,
) -> BoxedFilter<(impl Reply,)> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<BitcoinTransactionQuery>::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::<BitcoinBlockQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    if let Ok(zmq_endpoint) = endpoint.clone() {
        info!("Connect BitcoinZmqListener to {}", zmq_endpoint);

        let transaction_processor = DefaultBlockProcessor::new(
            transaction_query_repository.clone(),
            block_query_repository.clone(),
            transaction_query_result_repository.clone(),
            block_query_result_repository.clone(),
        );
        thread::spawn(move || {
            let bitcoind_zmq_listener =
                BitcoindZmqListener::new(zmq_endpoint.as_str(), transaction_processor);

            match bitcoind_zmq_listener {
                Ok(mut listener) => listener.start(),
                Err(e) => error!("Failed to start BitcoinZmqListener! {:?}", e),
            }
        });
    }

    let ledger_name = "bitcoin";

    let transaction_routes = route_factory.create(
        transaction_query_repository,
        transaction_query_result_repository,
        endpoint.clone(),
        ledger_name,
    );

    let block_routes = route_factory.create(
        block_query_repository,
        block_query_result_repository,
        endpoint,
        ledger_name,
    );

    transaction_routes.or(block_routes).boxed()
}

fn create_ethereum_routes(
    route_factory: &RouteFactory,
    endpoint: Result<String, VarError>,
) -> BoxedFilter<(impl Reply,)> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<EthereumTransactionQuery>::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::<EthereumBlockQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    if let Ok(web3_endpoint) = endpoint.clone() {
        info!("Starting EthereumSimpleListener on {}", web3_endpoint);

        let polling_wait_time = match var("ETHEREUM_POLLING_TIME_SEC") {
            Err(_) => 17,
            Ok(var) => var.parse().unwrap(),
        };
        let polling_wait_time = Duration::from_secs(polling_wait_time);

        let transaction_processor = DefaultBlockProcessor::new(
            transaction_query_repository.clone(),
            block_query_repository.clone(),
            transaction_query_result_repository.clone(),
            block_query_result_repository.clone(),
        );

        thread::spawn(move || {
            let web3_poller = EthereumWeb3BlockPoller::new(
                web3_endpoint.as_str(),
                polling_wait_time,
                transaction_processor,
            );
            match web3_poller {
                Ok(listener) => listener.start(),
                Err(e) => error!("Failed to start EthereumSimpleListener! {:?}", e),
            }
        });
    }

    let ledger_name = "ethereum";

    let transaction_routes = route_factory.create(
        transaction_query_repository,
        transaction_query_result_repository,
        endpoint.clone(),
        ledger_name,
    );

    let block_routes = route_factory.create(
        block_query_repository,
        block_query_result_repository,
        endpoint,
        ledger_name,
    );

    transaction_routes.or(block_routes).boxed()
}

fn load_settings() -> Settings {
    let config_path = var_or_default(
        "LEDGER_QUERY_SERVICE_CONFIG_PATH",
        "~/.config/ledger_query_service".into(),
    );
    let default_config = format!("{}/{}", config_path.trim(), "default");

    let settings = Settings::new(default_config);
    settings.unwrap()
}

fn var_or_default(name: &str, default: String) -> String {
    match var(name) {
        Ok(value) => {
            info!("Set {}={}", name, value);
            value
        }
        Err(_) => {
            eprintln!(
                "{} is not set, falling back to default: '{}' ",
                name, default
            );
            default
        }
    }
}
