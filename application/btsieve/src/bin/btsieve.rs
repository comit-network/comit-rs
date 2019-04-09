#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;

use bitcoin_rpc_client::{rpc::BlockchainInfo, BitcoinCoreClient, BitcoinRpcApi};
use bitcoin_support::Network as BitcoinNetwork;
use btsieve::{
    bitcoin::{self, bitcoind_zmq_listener::bitcoin_block_listener},
    ethereum::{self, ethereum_web3_block_poller::ethereum_block_listener},
    route_factory::create_endpoints,
    settings::{self, Settings},
    InMemoryQueryRepository, InMemoryQueryResultRepository, QueryMatch, QueryResultRepository,
    RouteFactory,
};
use config::ConfigError;
use directories;
use ethereum_support::{
    web3::{
        self,
        transports::{EventLoopHandle, Http},
        Web3,
    },
    Network as EthereumNetwork,
};
use futures::{future::Future, stream::Stream};
use std::{env::var, string::ToString, sync::Arc};
use tokio::runtime::Runtime;
use url::Url;
use warp::{self, filters::BoxedFilter, Filter, Reply};

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "Could not connect to ledger: {}", ledger)]
    ConnectionError { ledger: String },
    #[fail(display = "Unknown ledger network: {} for ledger {}", network, ledger)]
    UnknownLedgerVersion { network: String, ledger: String },
}

impl From<web3::Error> for Error {
    fn from(_e: web3::Error) -> Self {
        Error::ConnectionError {
            ledger: String::from("Ethereum"),
        }
    }
}

fn main() -> Result<(), failure::Error> {
    let _ = pretty_env_logger::try_init();

    let settings = load_settings()?;
    let mut runtime = tokio::runtime::Runtime::new()?;

    info!("Starting up with {:#?}", settings);

    let bitcoin_routes = create_bitcoin_routes(
        &mut runtime,
        settings.http_api.external_url.clone(),
        settings.bitcoin.unwrap(),
    )?;

    let (ethereum_routes, _event_loop_handle) = create_ethereum_routes(
        &mut runtime,
        settings.http_api.external_url.clone(),
        settings.ethereum.unwrap(),
    )?;

    let routes = bitcoin_routes.or(ethereum_routes);

    warp::serve(routes).run((settings.http_api.address_bind, settings.http_api.port_bind));
    Ok(())
}

fn create_bitcoin_routes(
    runtime: &mut Runtime,
    external_url: Url,
    settings: settings::Bitcoin,
) -> Result<BoxedFilter<(impl Reply,)>, Error> {
    let block_query_repository =
        Arc::new(InMemoryQueryRepository::<bitcoin::BlockQuery>::default());
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<bitcoin::TransactionQuery>::default());

    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let bitcoin_rpc_client = bitcoin_rpc_client::BitcoinCoreClient::new(
        settings.node_url.as_str(),
        settings.node_username.as_str(),
        settings.node_password.as_str(),
    );

    let blockchain_info = get_bitcoin_info(&bitcoin_rpc_client)?;
    info!("Connected to Bitcoin: {:?}", blockchain_info);
    let network = BitcoinNetwork::from(blockchain_info.chain).into();

    trace!("Setting up bitcoin routes to {:?}", network);

    info!("Connect BitcoinZmqListener to {}", settings.zmq_endpoint);

    {
        let block_query_repository = Arc::clone(&block_query_repository);
        let transaction_query_repository = Arc::clone(&transaction_query_repository);

        let block_query_result_repository = Arc::clone(&block_query_result_repository);
        let transaction_query_result_repository = Arc::clone(&transaction_query_result_repository);

        let blocks = bitcoin_block_listener(settings.zmq_endpoint.as_str())
            .expect("Should return a Bitcoind received for MinedBlocks");

        let bitcoin_processor = blocks.for_each(move |block| {
            bitcoin::check_block_queries(block_query_repository.clone(), block.clone()).for_each(
                |QueryMatch(id, block_id)| {
                    block_query_result_repository.add_result(id.0, block_id);
                },
            );

            bitcoin::check_transaction_queries(transaction_query_repository.clone(), block.clone())
                .for_each(|QueryMatch(id, block_id)| {
                    transaction_query_result_repository.add_result(id.0, block_id);
                });

            Ok(())
        });
        runtime.spawn(bitcoin_processor);
    }

    let client = Arc::new(bitcoin_rpc_client);

    let ledger_name = "bitcoin";

    let transaction_routes = create_endpoints::<bitcoin::queries::transaction::ReturnAs, _, _, _, _>(
        external_url.clone(),
        transaction_query_repository,
        transaction_query_result_repository,
        Arc::clone(&client),
        ledger_name,
        network,
    );

    let block_routes = create_endpoints::<bitcoin::queries::block::ReturnAs, _, _, _, _>(
        external_url.clone(),
        block_query_repository,
        block_query_result_repository,
        Arc::clone(&client),
        ledger_name,
        network,
    );

    Ok(transaction_routes.or(block_routes).boxed())
}

fn create_ethereum_routes(
    runtime: &mut Runtime,
    external_url: Url,
    settings: settings::Ethereum,
) -> Result<(BoxedFilter<(impl Reply,)>, EventLoopHandle), Error> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<ethereum::TransactionQuery>::default());
    let block_query_repository =
        Arc::new(InMemoryQueryRepository::<ethereum::BlockQuery>::default());
    let log_query_repository = Arc::new(InMemoryQueryRepository::<ethereum::EventQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let log_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    info!("Starting Ethereum Listener on {}", settings.node_url);

    let (event_loop, transport) =
        Http::new(settings.node_url.as_str()).expect("unable to connect to Ethereum node");
    let web3_client = Arc::new(Web3::new(transport));

    let network = get_ethereum_info(web3_client.clone())?.into();

    trace!("Setting up ethereum routes to {:?}", network);

    {
        let block_query_repository = block_query_repository.clone();
        let transaction_query_repository = transaction_query_repository.clone();
        let log_query_repository = log_query_repository.clone();

        let block_query_result_repository = block_query_result_repository.clone();
        let transaction_query_result_repository = transaction_query_result_repository.clone();
        let log_query_result_repository = log_query_result_repository.clone();

        let web3_client = web3_client.clone();

        let blocks = ethereum_block_listener(web3_client.clone(), settings.poll_interval_secs)
            .expect("Should return a Web3 block poller");

        let executor = runtime.executor();
        let web3_processor = blocks.for_each(move |block| {
            ethereum::check_block_queries(block_query_repository.clone(), block.clone()).for_each(
                |QueryMatch(id, block_id)| {
                    block_query_result_repository.add_result(id.0, block_id);
                },
            );

            ethereum::check_transaction_queries(
                transaction_query_repository.clone(),
                block.clone(),
            )
            .for_each(|QueryMatch(id, transaction_id)| {
                transaction_query_result_repository.add_result(id.0, transaction_id);
            });

            let log_query_result_repository = log_query_result_repository.clone();
            let log_query_future = ethereum::check_log_queries(
                log_query_repository.clone(),
                web3_client.clone(),
                block,
            )
            .for_each(move |QueryMatch(id, transaction_id)| {
                log_query_result_repository.add_result(id.0, transaction_id);
                Ok(())
            });

            executor.spawn(log_query_future);
            Ok(())
        });

        runtime.spawn(web3_processor);
    }

    let ledger_name = "ethereum";

    let transaction_routes = create_endpoints::<ethereum::queries::transaction::ReturnAs, _, _, _, _>(
        external_url.clone(),
        transaction_query_repository,
        transaction_query_result_repository,
        Arc::clone(&web3_client),
        ledger_name,
        network,
    );

    let block_routes = create_endpoints::<ethereum::queries::block::ReturnAs, _, _, _, _>(
        external_url.clone(),
        block_query_repository,
        block_query_result_repository,
        Arc::clone(&web3_client),
        ledger_name,
        network,
    );

    let bloom_routes = create_endpoints::<ethereum::queries::event::ReturnAs, _, _, _, _>(
        external_url.clone(),
        log_query_repository,
        log_query_result_repository,
        Arc::clone(&web3_client),
        ledger_name,
        network,
    );

    Ok((
        transaction_routes.or(block_routes).or(bloom_routes).boxed(),
        event_loop,
    ))
}

fn load_settings() -> Result<Settings, ConfigError> {
    match directories::UserDirs::new() {
        None => Err(config::ConfigError::Message(
            "Unable to determine user's home directory".to_string(),
        )),
        Some(dirs) => {
            let default_config = std::path::Path::join(dirs.home_dir(), ".config/btsieve");
            let comit_config_path = var_or_default(
                "BTSIEVE_CONFIG_PATH",
                default_config.to_string_lossy().to_string(),
            );
            let default_config = format!("{}/{}", comit_config_path.trim(), "default");
            let settings = Settings::create(default_config)?;
            Ok(settings)
        }
    }
}

fn get_bitcoin_info(client: &BitcoinCoreClient) -> Result<BlockchainInfo, Error> {
    client
        .get_blockchain_info()
        .map_err(|error| {
            error!(
                "Could not retrieve network version from ledger Bitcoin: {:?}",
                error
            );
            Error::ConnectionError {
                ledger: String::from("Bitcoin"),
            }
        })?
        .map_err(|error| {
            error!("Could not connect to ledger Bitcoin: {:?}", error);
            Error::ConnectionError {
                ledger: String::from("Bitcoin"),
            }
        })
}

fn get_ethereum_info(client: Arc<Web3<Http>>) -> Result<EthereumNetwork, Error> {
    let network = client.net().version().wait()?;
    trace!("Connected to ethereum {:?}", network);
    let network = EthereumNetwork::from_network_id(network);
    if network == EthereumNetwork::Unknown {
        return Err(Error::UnknownLedgerVersion {
            network: network.to_string(),
            ledger: String::from("Ethereum"),
        });
    }
    Ok(network)
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
