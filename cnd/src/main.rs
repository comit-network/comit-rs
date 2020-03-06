#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::print_stdout,
    clippy::dbg_macro
)]
#![forbid(unsafe_code)]
use crate::cli::Options;
use anyhow::Context;
use cnd::{
    btsieve::{
        bitcoin::{self, BitcoindConnector},
        ethereum::{self, Web3Connector},
    },
    config::{self, Settings},
    db::Sqlite,
    http_api::route_factory,
    load_swaps,
    network::Swarm,
    seed::RootSeed,
    swap_protocols::{state_store::InMemoryStateStore, Facade},
};
use rand::rngs::OsRng;
use std::{process, sync::Arc};
use structopt::StructOpt;
use tokio_compat::runtime;

mod cli;
mod trace;

fn main() -> anyhow::Result<()> {
    let options = cli::Options::from_args();

    if options.version {
        version();
        process::exit(0);
    }

    let settings = read_config(&options).and_then(Settings::from_config_file_and_defaults)?;

    if options.dump_config {
        dump_config(settings)?;
        process::exit(0);
    }

    crate::trace::init_tracing(settings.logging.level)?;

    let seed = RootSeed::from_dir_or_generate(&settings.data.dir, OsRng)?;

    let mut runtime = runtime::Builder::new()
        .stack_size(1024 * 1024 * 8) // the default is 2MB but that causes a segfault for some reason
        .build()?;

    const BITCOIN_BLOCK_CACHE_CAPACITY: usize = 144;
    let bitcoin_connector = {
        let config::Bitcoin { network, bitcoind } = settings.clone().bitcoin;
        bitcoin::Cache::new(
            BitcoindConnector::new(bitcoind.node_url, network)?,
            BITCOIN_BLOCK_CACHE_CAPACITY,
        )
    };

    const ETHEREUM_BLOCK_CACHE_CAPACITY: usize = 720;
    const ETHEREUM_RECEIPT_CACHE_CAPACITY: usize = 720;
    let ethereum_connector = ethereum::Cache::new(
        Web3Connector::new(settings.clone().ethereum.parity.node_url),
        ETHEREUM_BLOCK_CACHE_CAPACITY,
        ETHEREUM_RECEIPT_CACHE_CAPACITY,
    );

    let state_store = Arc::new(InMemoryStateStore::default());

    let database = Sqlite::new_in_dir(&settings.data.dir)?;

    let swarm = Swarm::new(
        &settings,
        seed,
        &mut runtime,
        &bitcoin_connector,
        &ethereum_connector,
        &state_store,
        &database,
    )?;

    let deps = Facade {
        bitcoin_connector,
        ethereum_connector,
        state_store: Arc::clone(&state_store),
        seed,
        swarm,
        db: database,
    };

    runtime.block_on_std(load_swaps::load_swaps_from_database(deps.clone()))?;
    runtime.spawn_std(spawn_warp_instance(settings, deps));

    // Block the current thread.
    ::std::thread::park();
    Ok(())
}

#[allow(clippy::print_stdout)] // We cannot use `log` before we have the config file
fn version() {
    let name: &'static str = "COMIT network daemon";
    let version: &'static str = env!("CARGO_PKG_VERSION");
    let commit: &'static str = env!("GIT_HASH");
    let length: usize = 12; // Abbreviate the hash, 12 digits is plenty.
    let short = &commit[..length];

    println!("{} {} ({})", name, version, short);
}

async fn spawn_warp_instance(settings: Settings, dependencies: Facade) {
    let routes = route_factory::create(dependencies, &settings.http_api.cors.allowed_origins);

    let listen_addr = settings.http_api.socket;

    tracing::info!("Starting HTTP server on {}", listen_addr);

    warp::serve(routes).bind(listen_addr).await
}

#[allow(clippy::print_stdout)] // We cannot use `log` before we have the config file
fn read_config(options: &Options) -> anyhow::Result<config::File> {
    // if the user specifies a config path, use it
    if let Some(path) = &options.config_file {
        eprintln!("Using config file {}", path.display());

        return config::File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    // try to load default config
    let default_path = cnd::default_config_path()?;

    if !default_path.exists() {
        return Ok(config::File::default());
    }

    eprintln!(
        "Using config file at default path: {}",
        default_path.display()
    );

    config::File::read(&default_path)
        .with_context(|| format!("failed to read config file {}", default_path.display()))
}

#[allow(clippy::print_stdout)] // Don't use the logger so its easier to cut'n'paste
fn dump_config(settings: Settings) -> anyhow::Result<()> {
    let file = config::File::from(settings);
    let serialized = toml::to_string(&file)?;
    println!("{}", serialized);
    Ok(())
}
