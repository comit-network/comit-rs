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
#![type_length_limit = "1049479"] // Regressed with Rust 1.46.0 :(

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

#[macro_use]
mod network;
#[cfg(test)]
mod proptest;
#[cfg(test)]
mod spectral_ext;
#[macro_use]
mod with_swap_types;
mod actions;
mod cli;
mod config;
mod connectors;
mod file_lock;
mod fs;
mod halbit;
mod hbit;
mod herc20;
mod http_api;
mod local_swap_id;
mod protocol_spawner;
mod respawn;
mod spawn;
mod state;
mod storage;
mod trace;
mod tracing_ext;
mod htlc_location {
    pub use comit::htlc_location::*;
}
mod identity {
    pub use comit::identity::*;
}
mod transaction {
    pub use comit::transaction::*;
}
mod asset {
    pub use comit::asset::*;
}
mod ethereum {
    pub use comit::ethereum::*;
}
mod bitcoin {
    pub use comit::bitcoin::*;
}
mod lightning {
    pub use comit::lightning::PublicKey;
}
mod btsieve {
    pub use comit::btsieve::*;
}

use self::{
    actions::*,
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    config::{validate_connection_to_network, Settings},
    connectors::Connectors,
    file_lock::TryLockExclusive,
    local_swap_id::LocalSwapId,
    network::{Swarm, SwarmWorker},
    protocol_spawner::{ProtocolSpawner, *},
    respawn::respawn,
    spawn::*,
    storage::{RootSeed, Sqlite, Storage},
};
use ::bitcoin::secp256k1::{All, Secp256k1};
use comit::{
    ledger, lnd::LndConnectorParams, LockProtocol, Never, RelativeTime, Role, Secret, SecretHash,
    Side, Timestamp,
};
use conquer_once::Lazy;
use rand::rngs::OsRng;
use std::{env, process};
use structopt::StructOpt;
use tokio::{net::TcpListener, runtime};

pub static SECP: Lazy<Secp256k1<All>> = Lazy::new(Secp256k1::new);

fn main() -> anyhow::Result<()> {
    let options = cli::Options::from_args();

    if options.version {
        version();
        process::exit(0);
    }

    let file = fs::read_config(&options)?;
    let settings = Settings::from_config_file_and_defaults(file, options.network)?;

    if options.dump_config {
        fs::dump_config(settings)?;
        process::exit(0);
    }

    crate::trace::init_tracing(settings.logging.level)?;
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!(
            "thread panicked at {}: {}",
            panic_info.location().expect("location is always present"),
            panic_info
                .payload()
                .downcast_ref::<String>()
                .unwrap_or(&String::from("no panic message"))
        )
    }));

    let database = Sqlite::new_in_dir(&settings.data.dir)?;

    let seed = RootSeed::from_dir_or_generate(&settings.data.dir, OsRng)?;

    let _locked_datadir = &settings.data.dir.try_lock_exclusive()?;

    let mut runtime = runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .build()?;

    let bitcoin_connector = {
        let config::Bitcoin { bitcoind, network } = &settings.bitcoin;
        let connector = BitcoindConnector::new(bitcoind.node_url.clone())?;

        runtime.block_on(async {
            match validate_connection_to_network(&connector, *network).await {
                Ok(Err(network_mismatch)) => Err(network_mismatch),
                Ok(Ok(())) => Ok(()),
                Err(e) => {
                    tracing::warn!("Could not validate Bitcoin node config: {}", e);
                    Ok(())
                }
            }
        })?;

        const BITCOIN_BLOCK_CACHE_CAPACITY: usize = 144;

        btsieve::bitcoin::Cache::new(connector, BITCOIN_BLOCK_CACHE_CAPACITY)
    };

    let ethereum_connector = {
        let config::Ethereum { geth, chain_id, .. } = &settings.ethereum;
        let connector = Web3Connector::new(geth.node_url.clone());

        runtime.block_on(async {
            match validate_connection_to_network(&connector, *chain_id).await {
                Ok(Err(network_mismatch)) => Err(network_mismatch),
                Ok(Ok(())) => Ok(()),
                Err(e) => {
                    tracing::warn!("Could not validate Ethereum node config: {}", e);
                    Ok(())
                }
            }
        })?;

        const ETHEREUM_BLOCK_CACHE_CAPACITY: usize = 720;
        const ETHEREUM_RECEIPT_CACHE_CAPACITY: usize = 720;

        btsieve::ethereum::Cache::new(
            connector,
            ETHEREUM_BLOCK_CACHE_CAPACITY,
            ETHEREUM_RECEIPT_CACHE_CAPACITY,
        )
    };

    let lnd_connector_params = LndConnectorParams::new(
        settings.lightning.lnd.rest_api_url.clone(),
        100,
        settings.lightning.lnd.cert_path.clone(),
        settings.lightning.lnd.readonly_macaroon_path.clone(),
    )
    .map_err(|err| {
        tracing::warn!(
            "Could not read initialise lnd configuration, halbit will not be available: {:?}",
            err
        );
    })
    .ok();

    let connectors = Connectors::new(bitcoin_connector, ethereum_connector);
    let storage = Storage::new(database, seed);

    let protocol_spawner = ProtocolSpawner::new(
        connectors.clone(),
        lnd_connector_params,
        runtime.handle().clone(),
        storage.clone(),
    );

    let swarm = runtime.block_on(Swarm::new(
        &settings,
        seed,
        runtime.handle().clone(),
        storage.clone(),
        protocol_spawner.clone(),
    ))?;

    let http_api_listener = runtime.block_on(bind_http_api_socket(&settings))?;
    match runtime.block_on(respawn(storage.clone(), protocol_spawner)) {
        Ok(()) => {}
        Err(e) => tracing::warn!("failed to respawn swaps: {:?}", e),
    };

    runtime.spawn(make_http_api_worker(
        settings,
        options.network.unwrap_or_default(),
        swarm.clone(),
        storage,
        connectors,
        http_api_listener,
    ));
    runtime.spawn(make_network_api_worker(swarm));

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

/// Binds to the socket for the HTTP API specified in the settings
///
/// Fails if we cannot bind to the socket.
/// We do this ourselves so we can shut down if this fails and don't just panic
/// some worker thread in tokio.
async fn bind_http_api_socket(settings: &Settings) -> anyhow::Result<tokio::net::TcpListener> {
    let listen_addr = settings.http_api.socket;
    let listener = TcpListener::bind(listen_addr).await?;

    Ok(listener)
}

/// Construct the worker that is going to process HTTP API requests.
async fn make_http_api_worker(
    settings: Settings,
    network: comit::Network,
    swarm: Swarm,
    storage: Storage,
    connectors: Connectors,
    incoming_requests: tokio::net::TcpListener,
) {
    let routes = http_api::create_routes(swarm, storage, connectors, &settings, network);

    match incoming_requests.local_addr() {
        Ok(socket) => {
            tracing::info!("Starting HTTP server on {} ...", socket);
            warp::serve(routes).serve_incoming(incoming_requests).await;
        }
        Err(e) => {
            tracing::error!("Cannot start HTTP server because {:?}", e);
        }
    }
}

/// Construct the worker that is going to process network (i.e. COMIT)
/// communication.
async fn make_network_api_worker(swarm: Swarm) {
    let worker = SwarmWorker { swarm };

    worker.await
}
