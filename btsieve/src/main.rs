#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use btsieve::{
    bitcoin::{self, bitcoind_http_blocksource::BitcoindHttpBlockSource},
    create_bitcoin_stub_endpoints, create_ethereum_stub_endpoints,
    ethereum::{self, web3_http_blocksource::Web3HttpBlockSource},
    expected_version_header,
    load_settings::load_settings,
    logging, route_factory, routes, settings, InMemoryQueryRepository,
    InMemoryQueryResultRepository,
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
use futures::future::Future;
use std::{string::ToString, sync::Arc};
use structopt::StructOpt;
use tokio::runtime::Runtime;
use warp::{self, filters::BoxedFilter, Filter, Reply};

mod cli;

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
    let options = cli::Options::from_args();
    let settings = load_settings(options.config_file)?;
    logging::set_up_logging(&settings);

    let mut runtime = tokio::runtime::Runtime::new()?;

    log::info!("Starting up with {:#?}", settings);

    let log = warp::log("btsieve::api");
    let ping_200 = warp::path("health").map(warp::reply);
    let ping_route = warp::get2().and(ping_200);

    match (settings.bitcoin, settings.ethereum) {
        (Some(bitcoin), Some(ethereum)) => {
            let (ethereum_routes, _event_loop) = create_ethereum_routes(&mut runtime, ethereum)?;
            warp::serve(
                expected_version_header::validate()
                    .and(
                        ping_route
                            .or(create_bitcoin_routes(bitcoin)?)
                            .or(ethereum_routes),
                    )
                    .recover(routes::customize_error)
                    .recover(expected_version_header::customize_error)
                    .with(log),
            )
            .run((settings.http_api.address_bind, settings.http_api.port_bind));
        }
        (Some(bitcoin), None) => {
            warp::serve(
                expected_version_header::validate()
                    .and(
                        ping_route
                            .or(create_bitcoin_routes(bitcoin)?)
                            .or(create_ethereum_stub_endpoints()),
                    )
                    .recover(routes::customize_error)
                    .recover(expected_version_header::customize_error)
                    .with(log),
            )
            .run((settings.http_api.address_bind, settings.http_api.port_bind));
        }
        (None, Some(ethereum)) => {
            let (ethereum_routes, _event_loop) = create_ethereum_routes(&mut runtime, ethereum)?;

            warp::serve(
                expected_version_header::validate()
                    .and(
                        ping_route
                            .or(create_bitcoin_stub_endpoints())
                            .or(ethereum_routes),
                    )
                    .recover(routes::customize_error)
                    .recover(expected_version_header::customize_error)
                    .with(log),
            )
            .run((settings.http_api.address_bind, settings.http_api.port_bind));
        }
        (None, None) => {
            warp::serve(
                expected_version_header::validate()
                    .and(
                        ping_route
                            .or(create_bitcoin_stub_endpoints())
                            .or(create_ethereum_stub_endpoints()),
                    )
                    .recover(routes::customize_error)
                    .recover(expected_version_header::customize_error)
                    .with(log),
            )
            .run((settings.http_api.address_bind, settings.http_api.port_bind));
        }
    }

    Ok(())
}

fn create_bitcoin_routes(settings: settings::Bitcoin) -> Result<BoxedFilter<(impl Reply,)>, Error> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<bitcoin::TransactionQuery>::default());

    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    let block_source = Arc::new(BitcoindHttpBlockSource::new(
        settings.node_url,
        settings.network,
    ));

    let ledger_name = "bitcoin";

    let transaction_routes = route_factory::create_endpoints::<
        bitcoin::queries::transaction::ReturnAs,
        _,
        _,
        _,
        _,
        BitcoindHttpBlockSource,
        _,
    >(
        transaction_query_repository,
        transaction_query_result_repository,
        block_source.clone(),
        block_source.clone(),
        ledger_name,
        settings.network.into(),
    );

    Ok(transaction_routes.boxed())
}

fn create_ethereum_routes(
    runtime: &mut Runtime,
    settings: settings::Ethereum,
) -> Result<(BoxedFilter<(impl Reply,)>, EventLoopHandle), Error> {
    let transaction_query_repository =
        Arc::new(InMemoryQueryRepository::<ethereum::TransactionQuery>::default());
    let log_query_repository = Arc::new(InMemoryQueryRepository::<ethereum::EventQuery>::default());
    let transaction_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());
    let log_query_result_repository = Arc::new(InMemoryQueryResultRepository::default());

    log::info!("Starting Ethereum Listener on {}", settings.node_url);

    let (event_loop, transport) =
        Http::new(settings.node_url.as_str()).expect("unable to connect to Ethereum node");
    let web3_client = Arc::new(Web3::new(transport));

    let network = get_ethereum_info(web3_client.clone())?;

    log::trace!("Setting up ethereum routes to {:?}", network);

    let web3_block_source =
        Arc::new(runtime.block_on(Web3HttpBlockSource::new(Arc::clone(&web3_client)))?);

    let ledger_name = "ethereum";

    let transaction_routes = route_factory::create_endpoints::<
        ethereum::queries::transaction::ReturnAs,
        _,
        _,
        _,
        _,
        Web3HttpBlockSource,
        _,
    >(
        transaction_query_repository,
        transaction_query_result_repository,
        Arc::clone(&web3_client),
        Arc::clone(&web3_block_source),
        ledger_name,
        network.into(),
    );

    let bloom_routes = route_factory::create_endpoints::<
        ethereum::queries::event::ReturnAs,
        _,
        _,
        _,
        _,
        Web3HttpBlockSource,
        _,
    >(
        log_query_repository,
        log_query_result_repository,
        Arc::clone(&web3_client),
        Arc::clone(&web3_block_source),
        ledger_name,
        network.into(),
    );

    Ok((transaction_routes.or(bloom_routes).boxed(), event_loop))
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
