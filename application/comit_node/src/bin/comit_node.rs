#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

#[macro_use]
extern crate log;
use directories;

use comit_node::{
    comit_client, comit_server,
    http_api::route_factory,
    ledger_query_service::DefaultLedgerQueryServiceApiClient,
    logging,
    settings::ComitNodeSettings,
    swap_protocols::{
        rfc003::{alice::AliceSpawner, bob::BobSpawner, state_store::InMemoryStateStore},
        InMemoryMetadataStore, LedgerEventDependencies, ProtocolDependencies, SwapId,
    },
};
use ethereum_support::*;
use std::{env::var, net::SocketAddr, sync::Arc};

// TODO: Make a nice command line interface here (using StructOpt f.e.) see #298
fn main() -> Result<(), failure::Error> {
    logging::set_up_logging();
    let settings = load_settings()?;

    // TODO: Maybe not print settings because of private keys?
    info!("Starting up with {:#?}", settings);

    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());
    let lqs_client = create_ledger_query_service_api_client(&settings);
    let comit_client_factory = Arc::new(comit_client::bam::BamClientPool::default());
    let dependencies = Arc::new(create_dependencies(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&lqs_client),
        comit_client_factory.clone(),
    ));

    let mut runtime = tokio::runtime::Runtime::new()?;

    spawn_warp_instance(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        dependencies.clone(),
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
            "could not determine user's home directory".to_string(),
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

fn create_dependencies<T, S, C>(
    settings: &ComitNodeSettings,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    lqs_client: Arc<DefaultLedgerQueryServiceApiClient>,
    comit_client_factory: Arc<dyn comit_client::ClientFactory<C>>,
) -> ProtocolDependencies<T, S, C> {
    ProtocolDependencies {
        ledger_events: LedgerEventDependencies {
            lqs_client,
            lqs_bitcoin_poll_interval: settings.ledger_query_service.bitcoin.poll_interval_secs,
            lqs_ethereum_poll_interval: settings.ledger_query_service.ethereum.poll_interval_secs,
        },
        metadata_store,
        state_store,
        comit_client_factory,
        seed: settings.comit.secret_seed,
        remote_comit_node: settings.comit.remote_comit_node_url,
    }
}

fn spawn_warp_instance<S: AliceSpawner>(
    settings: &ComitNodeSettings,
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    alice_spawner: Arc<S>,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(
        metadata_store,
        state_store,
        alice_spawner,
        settings.comit.secret_seed,
    );

    let server = warp::serve(routes).bind(SocketAddr::new(
        settings.http_api.address,
        settings.http_api.port,
    ));

    runtime.spawn(server);
}

fn spawn_comit_server<B: BobSpawner>(
    settings: &ComitNodeSettings,
    bob_spawner: Arc<B>,
    runtime: &mut tokio::runtime::Runtime,
) {
    runtime.spawn(
        comit_server::listen(settings.comit.comit_listen, bob_spawner).map_err(|e| {
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
