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
    settings::{self, Settings},
    BitcoindZmqListener, DefaultBlockProcessor, EthereumWeb3BlockPoller, InMemoryQueryRepository,
    InMemoryQueryResultRepository, RouteFactory,
};
use std::{env::var, sync::Arc, thread};
use warp::{filters::BoxedFilter, Filter, Reply};

fn main() {
    let _ = pretty_env_logger::try_init();

    let settings = load_settings();

    info!("Starting up with {:#?}", settings);

    let route_factory = RouteFactory::new(settings.http_api.external_url);

    let bitcoin_routes = create_bitcoin_routes(&route_factory, settings.bitcoin);

    let ethereum_routes = create_ethereum_routes(&route_factory, settings.ethereum);

    let routes = bitcoin_routes.or(ethereum_routes);
    warp::serve(routes).run((settings.http_api.address_bind, settings.http_api.port_bind));
}

fn create_bitcoin_routes(
    route_factory: &RouteFactory,
    settings: Option<settings::Bitcoin>,
) -> BoxedFilter<(impl Reply,)> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<BitcoinTransactionQuery>::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::<BitcoinBlockQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let client = match settings.clone() {
        Some(settings) => {
            let bitcoin_rpc_client = bitcoin_rpc_client::BitcoinCoreClient::new(
                settings.node_url.as_str(),
                settings.node_username.as_str(),
                settings.node_password.as_str(),
            );

            info!("Connect BitcoinZmqListener to {}", settings.zmq_endpoint);

            let transaction_processor = DefaultBlockProcessor::new(
                transaction_query_repository.clone(),
                block_query_repository.clone(),
                transaction_query_result_repository.clone(),
                block_query_result_repository.clone(),
            );
            thread::spawn(move || {
                let bitcoind_zmq_listener =
                    BitcoindZmqListener::new(settings.zmq_endpoint.as_str(), transaction_processor);

                match bitcoind_zmq_listener {
                    Ok(mut listener) => listener.start(),
                    Err(e) => error!("Failed to start BitcoinZmqListener! {:?}", e),
                }
            });

            Some(Arc::new(bitcoin_rpc_client))
        }
        // This is not great but could not find a better way with warp
        None => None,
    };

    let ledger_name = "bitcoin";

    let settings = settings.map(|_| ());

    let transaction_routes = route_factory.create(
        transaction_query_repository,
        transaction_query_result_repository,
        client.clone(),
        settings.clone(),
        ledger_name,
    );

    let block_routes = route_factory.create(
        block_query_repository,
        block_query_result_repository,
        None,
        settings,
        ledger_name,
    );

    transaction_routes.or(block_routes).boxed()
}

fn create_ethereum_routes(
    route_factory: &RouteFactory,
    settings: Option<settings::Ethereum>,
) -> BoxedFilter<(impl Reply,)> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<EthereumTransactionQuery>::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::<EthereumBlockQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    if let Some(settings) = settings.clone() {
        info!("Starting EthereumSimpleListener on {}", settings.node_url);

        let transaction_processor = DefaultBlockProcessor::new(
            transaction_query_repository.clone(),
            block_query_repository.clone(),
            transaction_query_result_repository.clone(),
            block_query_result_repository.clone(),
        );

        thread::spawn(move || {
            let web3_poller = EthereumWeb3BlockPoller::new(
                settings.node_url.as_str(),
                settings.poll_interval_secs,
                transaction_processor,
            );
            match web3_poller {
                Ok(listener) => listener.start(),
                Err(e) => error!("Failed to start EthereumSimpleListener! {:?}", e),
            }
        });
    }

    let ledger_name = "ethereum";

    let settings = settings.map(|_| ());

    let transaction_routes = route_factory.create(
        transaction_query_repository,
        transaction_query_result_repository,
        None,
        settings.clone(),
        ledger_name,
    );

    let block_routes = route_factory.create(
        block_query_repository,
        block_query_result_repository,
        None,
        settings,
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
