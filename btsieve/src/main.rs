#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use bitcoin_support::Network as BitcoinNetwork;
use bitcoincore_rpc::RpcApi;
use btsieve::{
    bitcoin::{self, bitcoind_zmq_listener::bitcoin_block_listener},
    ethereum::{self, ethereum_web3_block_poller::ethereum_block_listener},
    load_settings::{load_settings, Opt},
    logging, route_factory, settings, InMemoryQueryRepository, InMemoryQueryResultRepository,
    QueryMatch, QueryResultRepository,
};
use ethereum_support::{
    web3::{
        self,
        transports::{EventLoopHandle, Http},
        Web3,
    },
    Network as EthereumNetwork,
};
use failure::Fail;
use futures::{future::Future, stream::Stream};
use std::{string::ToString, sync::Arc};
use structopt::StructOpt;
use tokio::runtime::Runtime;
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
    let opt = Opt::from_args();

    let settings = load_settings(opt)?;
    logging::set_up_logging(&settings);

    let mut runtime = tokio::runtime::Runtime::new()?;

    log::info!("Starting up with {:#?}", settings);

    let bitcoin_routes = create_bitcoin_routes(&mut runtime, settings.bitcoin)?;

    let (ethereum_routes, _event_loop) = create_ethereum_routes(&mut runtime, settings.ethereum)?;

    let log = warp::log("btsieve::api");
    let ping_200 = warp::path("health").map(warp::reply);
    let ping_route = warp::get2().and(ping_200);

    let routes = ping_route.or(bitcoin_routes.or(ethereum_routes)).with(log);

    warp::serve(routes).run((settings.http_api.address_bind, settings.http_api.port_bind));
    Ok(())
}

fn create_bitcoin_routes(
    runtime: &mut Runtime,
    settings: Option<settings::Bitcoin>,
) -> Result<BoxedFilter<(impl Reply,)>, Error> {
    let block_query_repository =
        Arc::new(InMemoryQueryRepository::<bitcoin::BlockQuery>::default());
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<bitcoin::TransactionQuery>::default());

    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let mut bitcoin_chain = Bitcoin::default();

    let (client, network) = if let Some(settings) = settings {
        let bitcoin_rpc_client = bitcoincore_rpc::Client::new(
            settings.node_url.to_string(),
            settings.authentication.into(),
        )
        .map_err(|e| {
            log::debug!("failed to create bitcoincore_rpc::Client: {:?}", e);
            Error::ConnectionError {
                ledger: "bitcoin".to_owned(),
            }
        })?;
        let blockchain_info = get_bitcoin_info(&bitcoin_rpc_client)?;
        log::info!("Connected to Bitcoin: {:?}.", blockchain_info);
        let network = blockchain_info
            .chain
            .parse::<BitcoinNetwork>()
            .map_err(|_| Error::UnknownLedgerVersion {
                network: blockchain_info.chain,
                ledger: "bitcoin".to_string(),
            })?
            .into();

        log::trace!("Setting up bitcoin routes to {:?}.", network);

        log::info!("Connect BitcoinZmqListener to {}.", settings.zmq_endpoint);

        {
            let block_query_repository = Arc::clone(&block_query_repository);
            let transaction_query_repository = Arc::clone(&transaction_query_repository);

            let block_query_result_repository = Arc::clone(&block_query_result_repository);
            let transaction_query_result_repository =
                Arc::clone(&transaction_query_result_repository);

            let blocks = bitcoin_block_listener(settings.zmq_endpoint.as_str())
                .expect("Should return a Bitcoind received for MinedBlocks");

            let bitcoin_processor = blocks.for_each(move |block| {
                bitcoin_chain.add_block(block.clone());

                bitcoin::check_block_queries(block_query_repository.clone(), block.clone())
                    .for_each(|QueryMatch(id, block_id)| {
                        block_query_result_repository.add_result(id.0, block_id);
                    });

                bitcoin::check_transaction_queries(
                    transaction_query_repository.clone(),
                    block.clone(),
                )
                .for_each(|QueryMatch(id, block_id)| {
                    transaction_query_result_repository.add_result(id.0, block_id);
                });

                Ok(())
            });
            runtime.spawn(bitcoin_processor);
        }
        (Some(Arc::from(bitcoin_rpc_client)), Some(network))
    } else {
        (None, None)
    };

    let ledger_name = "bitcoin";

    let transaction_routes =
        route_factory::create_endpoints::<bitcoin::queries::transaction::ReturnAs, _, _, _, _>(
            transaction_query_repository,
            transaction_query_result_repository,
            client.clone(),
            ledger_name,
            network,
        );

    let block_routes =
        route_factory::create_endpoints::<bitcoin::queries::block::ReturnAs, _, _, _, _>(
            block_query_repository,
            block_query_result_repository,
            client,
            ledger_name,
            network,
        );

    Ok(transaction_routes.or(block_routes).boxed())
}

fn create_ethereum_routes(
    runtime: &mut Runtime,
    settings: Option<settings::Ethereum>,
) -> Result<(BoxedFilter<(impl Reply,)>, Option<EventLoopHandle>), Error> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<ethereum::TransactionQuery>::default());
    let block_query_repository =
        Arc::new(InMemoryQueryRepository::<ethereum::BlockQuery>::default());
    let log_query_repository = Arc::new(InMemoryQueryRepository::<ethereum::EventQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let block_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let log_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let (client, network, event_loop) = if let Some(settings) = settings {
        log::info!("Starting Ethereum Listener on {}", settings.node_url);

        let (event_loop, transport) =
            Http::new(settings.node_url.as_str()).expect("unable to connect to Ethereum node");
        let web3_client = Arc::new(Web3::new(transport));

        let network = get_ethereum_info(web3_client.clone())?.into();

        log::trace!("Setting up ethereum routes to {:?}", network);

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
                ethereum::check_block_queries(block_query_repository.clone(), block.clone())
                    .for_each(|QueryMatch(id, block_id)| {
                        block_query_result_repository.add_result(id.0, block_id);
                    });

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
        (Some(web3_client), Some(network), Some(event_loop))
    } else {
        (None, None, None)
    };

    let ledger_name = "ethereum";

    let transaction_routes =
        route_factory::create_endpoints::<ethereum::queries::transaction::ReturnAs, _, _, _, _>(
            transaction_query_repository,
            transaction_query_result_repository,
            client.clone(),
            ledger_name,
            network,
        );

    let block_routes =
        route_factory::create_endpoints::<ethereum::queries::block::ReturnAs, _, _, _, _>(
            block_query_repository,
            block_query_result_repository,
            client.clone(),
            ledger_name,
            network,
        );

    let bloom_routes =
        route_factory::create_endpoints::<ethereum::queries::event::ReturnAs, _, _, _, _>(
            log_query_repository,
            log_query_result_repository,
            client.clone(),
            ledger_name,
            network,
        );

    Ok((
        transaction_routes.or(block_routes).or(bloom_routes).boxed(),
        event_loop,
    ))
}

fn get_bitcoin_info(
    client: &bitcoincore_rpc::Client,
) -> Result<bitcoincore_rpc::json::GetBlockchainInfoResult, Error> {
    client.get_blockchain_info().map_err(|error| {
        log::error!(
            "Could not retrieve network version from ledger Bitcoin: {:?}",
            error
        );
        Error::ConnectionError {
            ledger: String::from("Bitcoin"),
        }
    })
}

fn get_ethereum_info(client: Arc<Web3<Http>>) -> Result<EthereumNetwork, Error> {
    let network = client.net().version().wait()?;
    log::trace!("Connected to ethereum {:?}", network);
    let network = EthereumNetwork::from_network_id(network);
    if network == EthereumNetwork::Unknown {
        return Err(Error::UnknownLedgerVersion {
            network: network.to_string(),
            ledger: String::from("Ethereum"),
        });
    }
    Ok(network)
}
