#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate log;

use comit_node::{
    comit_server,
    connection_pool::ConnectionPool,
    http_api::route_factory,
    ledger_query_service::DefaultLedgerQueryServiceApiClient,
    logging,
    settings::ComitNodeSettings,
    swap_protocols::{
        metadata_store::MetadataStore,
        rfc003::state_store::{InMemoryStateStore, StateStore},
        InMemoryMetadataStore, LedgerEventDependencies, ProtocolDependencies, SwapId,
    },
};
use directories;
use ethereum_support::*;
use std::{env::var, net::SocketAddr, sync::Arc};

// TODO: Make a nice command line interface here (using StructOpt f.e.) see #298
fn main() -> Result<(), failure::Error> {
    logging::set_up_logging();
    let settings = load_settings()?;

    info!("Starting up with {:#?}", settings);

    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());
    let lqs_client = create_ledger_query_service_api_client(&settings);
    let connection_pool = Arc::new(ConnectionPool::default());
    let dependencies = create_dependencies(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&lqs_client),
        Arc::clone(&connection_pool),
    );

    let mut runtime = tokio::runtime::Runtime::new()?;

    spawn_warp_instance(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        dependencies.clone(),
        Arc::clone(&connection_pool),
        &mut runtime,
    );

    spawn_comit_server(&settings, dependencies.clone(), &mut runtime);

    // Block the current thread.
    ::std::thread::park();
    Ok(())
}

fn load_settings() -> Result<ComitNodeSettings, config::ConfigError> {
    match directories::UserDirs::new() {
        None => Err(config::ConfigError::Message(
            "Unable to determine user's home directory".to_string(),
        )),
        Some(dirs) => {
            let default_config = std::path::Path::join(dirs.home_dir(), ".config/comit_node");
            let comit_config_path = var_or_default(
                "COMIT_NODE_CONFIG_PATH",
                default_config.to_string_lossy().to_string(),
            );
            let run_mode_config = var_or_default("RUN_MODE", "development".into());
            let default_config = format!("{}/{}", comit_config_path.trim(), "default");
            let run_mode_config = format!("{}/{}", comit_config_path.trim(), run_mode_config);
            let settings = ComitNodeSettings::create(default_config, run_mode_config)?;
            Ok(settings)
        }
    }
}

fn create_ledger_query_service_api_client(
    settings: &ComitNodeSettings,
) -> Arc<DefaultLedgerQueryServiceApiClient> {
    Arc::new(DefaultLedgerQueryServiceApiClient::new(
        &settings.ledger_query_service.url,
    ))
}

fn create_dependencies<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    settings: &ComitNodeSettings,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
    connection_pool: Arc<ConnectionPool>,
) -> ProtocolDependencies<T, S> {
    ProtocolDependencies {
        ledger_events: LedgerEventDependencies {
            lqs_client,
            lqs_bitcoin_poll_interval: settings.ledger_query_service.bitcoin.poll_interval_secs,
            lqs_ethereum_poll_interval: settings.ledger_query_service.ethereum.poll_interval_secs,
        },
        metadata_store,
        state_store,
        connection_pool,
        seed: settings.comit.secret_seed,
    }
}

fn spawn_warp_instance<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    settings: &ComitNodeSettings,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    protocol_dependencies: ProtocolDependencies<T, S>,
    connection_pool: Arc<ConnectionPool>,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(
        metadata_store,
        state_store,
        protocol_dependencies,
        settings.comit.secret_seed,
        connection_pool,
    );

    let listen_addr = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    info!("Starting HTTP server on {:?}", listen_addr);

    let server = warp::serve(routes).bind(listen_addr);

    runtime.spawn(server);
}

fn spawn_comit_server<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    settings: &ComitNodeSettings,
    protocol_dependencies: ProtocolDependencies<T, S>,
    runtime: &mut tokio::runtime::Runtime,
) {
    runtime.spawn(
        comit_server::listen(settings.comit.comit_listen, protocol_dependencies).map_err(|e| {
            error!("ComitServer shutdown: {:?}", e);
        }),
    );
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
