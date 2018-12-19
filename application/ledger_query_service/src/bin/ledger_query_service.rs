#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate log;

use ethereum_support::web3::{
    transports::{EventLoopHandle, Http},
    Web3,
};
use futures::stream::Stream;
use ledger_query_service::{
    bitcoin::{BitcoinBlockQuery, BitcoinTransactionQuery},
    ethereum::{EthereumBlockQuery, EthereumTransactionQuery},
    settings::{self, Settings},
    BitcoindZmqListener, BlockProcessor, DefaultBlockProcessor, EthereumWeb3BlockPoller,
    InMemoryQueryRepository, InMemoryQueryResultRepository, QueryResultRepository, RouteFactory,
};
use std::{env::var, sync::Arc};
use tokio::runtime::Runtime;
use warp::{self, filters::BoxedFilter, Filter, Reply};

fn main() {
    let _ = pretty_env_logger::try_init();

    let settings = load_settings();
    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    info!("Starting up with {:#?}", settings);

    let route_factory = RouteFactory::new(settings.http_api.external_url);

    let bitcoin_routes = create_bitcoin_routes(&mut runtime, &route_factory, settings.bitcoin);

    let (ethereum_routes, _event_loop_handle) =
        create_ethereum_routes(&mut runtime, &route_factory, settings.ethereum);

    let routes = bitcoin_routes.or(ethereum_routes);

    warp::serve(routes).run((settings.http_api.address_bind, settings.http_api.port_bind));
}

fn create_bitcoin_routes(
    runtime: &mut Runtime,
    route_factory: &RouteFactory,
    settings: settings::Bitcoin,
) -> BoxedFilter<(impl Reply,)> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<BitcoinTransactionQuery>::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::<BitcoinBlockQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let bitcoin_rpc_client = bitcoin_rpc_client::BitcoinCoreClient::new(
        settings.node_url.as_str(),
        settings.node_username.as_str(),
        settings.node_password.as_str(),
    );

    info!("Connect BitcoinZmqListener to {}", settings.zmq_endpoint);

    let mut transaction_processor = DefaultBlockProcessor::new(
        transaction_query_repository.clone(),
        block_query_repository.clone(),
        transaction_query_result_repository.clone(),
        block_query_result_repository.clone(),
    );

    {
        let transaction_query_result_repository = transaction_query_result_repository.clone();
        let block_query_result_repository = block_query_result_repository.clone();

        let bitcoin_blocks = BitcoindZmqListener::create(settings.zmq_endpoint.as_str())
            .expect("Should return a Bitcoind received for MinedBlocks");
        let bitcoin_processor = bitcoin_blocks
            .and_then(move |block| transaction_processor.process(block))
            .for_each(move |(block_results, transaction_results)| {
                for (id, block_id) in block_results {
                    block_query_result_repository.add_result(id, block_id);
                }
                for (id, tx_id) in transaction_results {
                    transaction_query_result_repository.add_result(id, tx_id);
                }
                Ok(())
            });
        runtime.spawn(bitcoin_processor);
    }

    let client = Arc::new(bitcoin_rpc_client);

    let ledger_name = "bitcoin";

    let transaction_routes = route_factory.create(
        transaction_query_repository,
        transaction_query_result_repository,
        Some(Arc::clone(&client)),
        ledger_name,
    );

    let block_routes = route_factory.create(
        block_query_repository,
        block_query_result_repository,
        None,
        ledger_name,
    );

    transaction_routes.or(block_routes).boxed()
}

fn create_ethereum_routes(
    runtime: &mut Runtime,
    route_factory: &RouteFactory,
    settings: settings::Ethereum,
) -> (BoxedFilter<(impl Reply,)>, EventLoopHandle) {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<EthereumTransactionQuery>::default());
    let block_query_repository = Arc::new(InMemoryQueryRepository::<EthereumBlockQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    info!("Starting EthereumSimpleListener on {}", settings.node_url);

    let mut transaction_processor = DefaultBlockProcessor::new(
        transaction_query_repository.clone(),
        block_query_repository.clone(),
        transaction_query_result_repository.clone(),
        block_query_result_repository.clone(),
    );

    let (event_loop, transport) =
        Http::new(settings.node_url.as_str()).expect("unable to connect to Ethereum node");
    let web3_client = Arc::new(Web3::new(transport));

    let poller_client = Arc::clone(&web3_client);

    {
        let transaction_query_result_repository = transaction_query_result_repository.clone();
        let block_query_result_repository = block_query_result_repository.clone();

        let web3_blocks =
            EthereumWeb3BlockPoller::create(poller_client, settings.poll_interval_secs)
                .expect("Should return a Web3 block poller");
        let web3_processor = web3_blocks
            .and_then(move |block| transaction_processor.process(block))
            .for_each(move |(block_results, transaction_results)| {
                for (id, block_id) in block_results {
                    block_query_result_repository.add_result(id, block_id);
                }
                for (id, tx_id) in transaction_results {
                    transaction_query_result_repository.add_result(id, tx_id);
                }
                Ok(())
            });
        runtime.spawn(web3_processor);
    }

    let ledger_name = "ethereum";

    let transaction_routes = route_factory.create(
        transaction_query_repository,
        transaction_query_result_repository,
        Some(Arc::clone(&web3_client)),
        ledger_name,
    );

    let block_routes = route_factory.create(
        block_query_repository,
        block_query_result_repository,
        None,
        ledger_name,
    );

    (transaction_routes.or(block_routes).boxed(), event_loop)
}

fn load_settings() -> Settings {
    let config_path = match var("LEDGER_QUERY_SERVICE_CONFIG_PATH") {
        Ok(value) => value,
        Err(_) => "~/.config/ledger_query_service".into(),
    };
    info!("Using settings located in {}", config_path);
    let default_config = format!("{}/{}", config_path.trim(), "default");

    let settings = Settings::create(default_config);
    settings.unwrap()
}
