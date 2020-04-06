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
    config::{self, validation::validate_blockchain_config, Settings},
    db::Sqlite,
    file_lock::TryLockExclusive,
    http_api::route_factory,
    jsonrpc,
    lnd::LndConnectorParams,
    load_swaps,
    network::Swarm,
    seed::RootSeed,
    swap_protocols::{
        halight::InvoiceStates, Facade, Facade2, LedgerStates, SwapCommunicationStates,
        SwapErrorStates,
    },
};

use rand::rngs::OsRng;
use std::{process, sync::Arc};
use structopt::StructOpt;
use tokio::runtime;
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

    let database = Sqlite::new_in_dir(&settings.data.dir)?;

    let seed = RootSeed::from_dir_or_generate(&settings.data.dir, OsRng)?;

    let _locked_datadir = &settings.data.dir.try_lock_exclusive()?;

    let mut runtime = runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .thread_stack_size(1024 * 1024 * 8) // the default is 2MB but that causes a segfault for some reason
        .build()?;

    let bitcoin_connector = {
        let config::Bitcoin { bitcoind, network } = &settings.bitcoin;
        let connector = BitcoindConnector::new(bitcoind.node_url.clone(), *network)?;

        runtime.block_on(async {
            validate_blockchain_config(&connector, *network)
                .await
                .or_else::<anyhow::Error, _>(|e| {
                    let conn_error = e.downcast::<reqwest::Error>()?;
                    tracing::warn!("Could not validate Bitcoin node config: {}", conn_error);

                    Ok(())
                })
        })?;

        const BITCOIN_BLOCK_CACHE_CAPACITY: usize = 144;

        Arc::new(bitcoin::Cache::new(connector, BITCOIN_BLOCK_CACHE_CAPACITY))
    };

    let ethereum_connector = {
        let config::Ethereum { parity, chain_id } = &settings.ethereum;
        let connector = Web3Connector::new(parity.node_url.clone());

        runtime.block_on(async {
            validate_blockchain_config(&connector, *chain_id)
                .await
                .or_else::<anyhow::Error, _>(|e| {
                    let conn_error = e.downcast::<jsonrpc::Error>()?;
                    tracing::warn!("Could not validate Ethereum node config: {}", conn_error);

                    Ok(())
                })
        })?;

        const ETHEREUM_BLOCK_CACHE_CAPACITY: usize = 720;
        const ETHEREUM_RECEIPT_CACHE_CAPACITY: usize = 720;

        Arc::new(ethereum::Cache::new(
            connector,
            ETHEREUM_BLOCK_CACHE_CAPACITY,
            ETHEREUM_RECEIPT_CACHE_CAPACITY,
        ))
    };

    let lnd_connector_params = LndConnectorParams {
        lnd_url: settings.lightning.lnd.rest_api_url.clone(),
        retry_interval_ms: 100,
        certificate_path: settings.lightning.lnd.cert_path.clone(),
        macaroon_path: settings.lightning.lnd.readonly_macaroon_path.clone(),
    };

    // Han protocol
    let alpha_ledger_state = Arc::new(LedgerStates::default());
    let beta_ledger_state = Arc::new(LedgerStates::default());
    let swap_communication_states = Arc::new(SwapCommunicationStates::default());

    // HALight
    let invoice_states = Arc::new(InvoiceStates::default());

    let swap_error_states = Arc::new(SwapErrorStates::default());

    let swarm = Swarm::new(
        &settings,
        seed,
        &mut runtime,
        Arc::clone(&bitcoin_connector),
        Arc::clone(&ethereum_connector),
        lnd_connector_params,
        Arc::clone(&swap_communication_states),
        Arc::clone(&alpha_ledger_state),
        Arc::clone(&beta_ledger_state),
        Arc::clone(&invoice_states),
        &database,
    )?;

    let facade2 = Facade2 {
        swarm: swarm.clone(),
        alpha_ledger_state: Arc::clone(&alpha_ledger_state),
        beta_ledger_state: Arc::clone(&invoice_states),
    };

    let deps = Facade {
        bitcoin_connector,
        ethereum_connector,
        alpha_ledger_state,
        beta_ledger_state,
        swap_communication_states,
        swap_error_states,
        seed,
        db: database,
        swarm,
    };

    runtime.block_on(load_swaps::load_swaps_from_database(deps.clone()))?;
    runtime.spawn(spawn_warp_instance(settings, deps, facade2));

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

async fn spawn_warp_instance(settings: Settings, dependencies: Facade, facade2: Facade2) {
    let routes = route_factory::create(
        dependencies,
        facade2,
        &settings.http_api.cors.allowed_origins,
    );

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
