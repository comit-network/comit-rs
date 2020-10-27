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
mod bitcoin_fees;
mod cli;
mod config;
mod connectors;
mod file_lock;
mod fs;
mod hbit;
mod herc20;
mod http_api;
mod local_swap_id;
mod republish;
mod respawn;
mod spawn;
mod state;
mod storage;
mod trace;

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
mod btsieve {
    pub use comit::btsieve::*;
}

use self::{
    actions::*,
    bitcoin_fees::BitcoinFees,
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    config::{validate_connection_to_network, Settings},
    connectors::Connectors,
    file_lock::TryLockExclusive,
    local_swap_id::LocalSwapId,
    network::{Swarm, SwarmWorker},
    republish::republish_open_orders,
    respawn::respawn,
    spawn::*,
    storage::{RootSeed, Sqlite, Storage},
};
use crate::{
    cli::{Command, CreateTransaction},
    storage::{Load, SwapContext},
};
use ::bitcoin::secp256k1::{All, Secp256k1};
use anyhow::{Context, Result};
use comit::{ledger, LockProtocol, Never, Role, Secret, SecretHash, Side, Timestamp};
use conquer_once::Lazy;
use futures::future;
use rand::rngs::OsRng;
use std::{env, process};
use structopt::StructOpt;
use tokio::{net::TcpListener, runtime::Handle};

pub static SECP: Lazy<Secp256k1<All>> = Lazy::new(Secp256k1::new);

#[tokio::main]
async fn main() -> Result<()> {
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
    let storage = Storage::new(database, seed);

    let _locked_datadir = &settings.data.dir.try_lock_exclusive()?;

    let bitcoin_fees = match &settings.bitcoin.fees {
        config::BitcoinFees::StaticSatPerVbyte(fee) => BitcoinFees::static_rate(*fee),
        config::BitcoinFees::CypherBlock(url) => {
            BitcoinFees::block_cypher(url.clone(), options.network.unwrap_or_default())
        }
    };

    #[allow(clippy::print_stdout)] // The point of these sub-commands is to print to stdout.
    if let Some(cmd) = options.cmd {
        let to_print = execute_subcommand(cmd, &storage, &bitcoin_fees)
            .await
            .context("failed to execute subcommand")?;

        println!("{}", to_print);
        return Ok(());
    }

    let bitcoin_connector = {
        let config::Bitcoin {
            bitcoind,
            network,
            fees: _,
        } = &settings.bitcoin;
        let connector = BitcoindConnector::new(bitcoind.node_url.clone())?;

        match validate_connection_to_network(&connector, *network).await {
            Ok(inner) => inner?,
            Err(e) => tracing::warn!("Could not validate Bitcoin node config: {}", e),
        }

        const BITCOIN_BLOCK_CACHE_CAPACITY: usize = 144;

        btsieve::bitcoin::Cache::new(connector, BITCOIN_BLOCK_CACHE_CAPACITY)
    };

    let ethereum_connector = {
        let config::Ethereum { geth, chain_id, .. } = &settings.ethereum;
        let connector = Web3Connector::new(geth.node_url.clone());

        match validate_connection_to_network(&connector, *chain_id).await {
            Ok(inner) => inner?,
            Err(e) => tracing::warn!("Could not validate Ethereum node config: {}", e),
        }

        const ETHEREUM_BLOCK_CACHE_CAPACITY: usize = 720;
        const ETHEREUM_RECEIPT_CACHE_CAPACITY: usize = 720;

        btsieve::ethereum::Cache::new(
            connector,
            ETHEREUM_BLOCK_CACHE_CAPACITY,
            ETHEREUM_RECEIPT_CACHE_CAPACITY,
        )
    };

    let connectors = Connectors::new(bitcoin_connector, ethereum_connector);

    let swarm = Swarm::new(
        &settings,
        seed,
        Handle::current(),
        storage.clone(),
        connectors.clone(),
    )
    .await?;

    let http_api_listener = bind_http_api_socket(&settings).await?;
    match respawn(storage.clone(), connectors.clone(), Handle::current()).await {
        Ok(()) => {}
        Err(e) => tracing::warn!("failed to respawn swaps: {:#}", e),
    };
    match republish_open_orders(storage.clone(), swarm.clone()).await {
        Ok(()) => {}
        Err(e) => tracing::warn!("failed to republish orders: {:#}", e),
    };

    tokio::spawn(make_http_api_worker(
        settings,
        bitcoin_fees,
        options.network.unwrap_or_default(),
        swarm.clone(),
        storage,
        connectors,
        http_api_listener,
    ));
    tokio::spawn(make_network_api_worker(swarm));

    future::pending::<()>().await;

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
async fn bind_http_api_socket(settings: &Settings) -> Result<tokio::net::TcpListener> {
    let listen_addr = settings.http_api.socket;
    let listener = TcpListener::bind(listen_addr).await?;

    Ok(listener)
}

/// Construct the worker that is going to process HTTP API requests.
async fn make_http_api_worker(
    settings: Settings,
    bitcoin_fees: BitcoinFees,
    network: comit::Network,
    swarm: Swarm,
    storage: Storage,
    connectors: Connectors,
    incoming_requests: tokio::net::TcpListener,
) {
    let routes =
        http_api::create_routes(swarm, storage, connectors, &settings, bitcoin_fees, network);

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

async fn execute_subcommand(
    cmd: Command,
    storage: &Storage,
    bitcoin_fees: &BitcoinFees,
) -> Result<String> {
    match cmd {
        Command::PrintSecret { swap_id } => {
            let swap_context: SwapContext = storage
                .load(swap_id)
                .await
                .with_context(|| format!("failed to load swap {} from database", swap_id))?;

            if let Role::Bob = swap_context.role {
                anyhow::bail!(
                    "We are Bob for swap {} and Bob doesn't choose the secret",
                    swap_id
                )
            }

            let secret = storage.seed.derive_swap_seed(swap_id).derive_secret();

            Ok(format!("{:x}", secret))
        }
        Command::CreateTransaction(CreateTransaction::Redeem {
            swap_id,
            secret,
            outpoint,
            address,
        }) => {
            let swap_context: SwapContext = storage
                .load(swap_id)
                .await
                .with_context(|| format!("failed to load swap {} from database", swap_id))?;

            let secret = match (secret, swap_context.role) {
                (Some(_), Role::Alice) => anyhow::bail!(
                    "We are Alice for swap {}, no need to provide a secret on the commandline",
                    swap_id
                ),
                (None, Role::Bob) => anyhow::bail!(
                    "We are Bob for swap {}, please provide the secret for this swap with --secret",
                    swap_id
                ),
                (Some(secret), Role::Bob) => secret,
                (None, Role::Alice) => storage.seed.derive_swap_seed(swap_id).derive_secret(),
            };

            let hbit_params = match swap_context {
                SwapContext {
                    alpha: LockProtocol::Hbit,
                    beta: LockProtocol::Herc20,
                    role: Role::Bob,
                    ..
                } => {
                    let swap: Swap<hbit::Params, herc20::Params> = storage.load(swap_id).await?;

                    swap.alpha
                }
                SwapContext {
                    alpha: LockProtocol::Herc20,
                    beta: LockProtocol::Hbit,
                    role: Role::Alice,
                    ..
                } => {
                    let swap: Swap<herc20::Params, hbit::Params> = storage.load(swap_id).await?;

                    swap.beta
                }
                _ => {
                    anyhow::bail!("Swap {} does either not involve hbit or we are not in the correct role to redeem it")
                }
            };

            let transaction = hbit_params.build_redeem_action(
                &*SECP,
                hbit_params.asset,
                outpoint,
                storage
                    .seed
                    .derive_swap_seed(swap_id)
                    .derive_transient_redeem_identity(),
                address,
                secret,
                bitcoin_fees.get_per_vbyte_rate().await?,
            )?;

            Ok(hex::encode(::bitcoin::consensus::serialize(
                &transaction.transaction,
            )))
        }
        Command::CreateTransaction(CreateTransaction::Refund {
            swap_id,
            outpoint,
            address,
        }) => {
            let swap_context: SwapContext = storage
                .load(swap_id)
                .await
                .with_context(|| format!("failed to load swap {} from database", swap_id))?;

            let hbit_params = match swap_context {
                SwapContext {
                    alpha: LockProtocol::Hbit,
                    beta: LockProtocol::Herc20,
                    role: Role::Bob,
                    ..
                } => {
                    let swap: Swap<hbit::Params, herc20::Params> = storage.load(swap_id).await?;

                    swap.alpha
                }
                SwapContext {
                    alpha: LockProtocol::Herc20,
                    beta: LockProtocol::Hbit,
                    role: Role::Alice,
                    ..
                } => {
                    let swap: Swap<herc20::Params, hbit::Params> = storage.load(swap_id).await?;

                    swap.beta
                }
                _ => {
                    anyhow::bail!("Swap {} does either not involve hbit or we are not in the correct role to redeem it")
                }
            };

            let transaction = hbit_params.build_refund_action(
                &*SECP,
                hbit_params.asset,
                outpoint,
                storage
                    .seed
                    .derive_swap_seed(swap_id)
                    .derive_transient_redeem_identity(),
                address,
                bitcoin_fees.get_per_vbyte_rate().await?,
            )?;

            Ok(hex::encode(::bitcoin::consensus::serialize(
                &transaction.transaction,
            )))
        }
    }
}
