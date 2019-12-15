#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]
use crate::cli::Options;
use anyhow::Context;
use cnd::{
    btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector},
    config::{self, Settings},
    db::Sqlite,
    http_api::route_factory,
    load_swaps,
    network::{self, transport, Network},
    seed::Seed,
    swap_protocols::{rfc003::state_store::InMemoryStateStore, Facade},
};
use futures::{stream, Future, Stream};
use futures_core::{FutureExt, TryFutureExt};
use libp2p::{
    identity::{self, ed25519},
    PeerId, Swarm,
};
use rand::rngs::OsRng;
use std::{
    net::SocketAddr,
    process,
    sync::{Arc, Mutex},
};
use structopt::StructOpt;

mod cli;
mod logging;

fn main() -> anyhow::Result<()> {
    let options = cli::Options::from_args();

    let settings = read_config(&options).and_then(Settings::from_config_file_and_defaults)?;

    if options.dump_config {
        dump_config(settings)?;
        process::exit(0);
    }

    let base_log_level = settings.logging.level;
    logging::initialize(base_log_level, settings.logging.structured)?;

    let seed = Seed::from_dir_or_generate(&settings.data.dir, OsRng)?;

    let mut runtime = tokio::runtime::Runtime::new()?;

    let bitcoin_connector = {
        let config::Bitcoin { node_url, network } = settings.clone().bitcoin;
        BitcoindConnector::new(node_url, network)?
    };

    let (ethereum_connector, _event_loop_handle) =
        { Web3Connector::new(settings.clone().ethereum.node_url, runtime.executor())? };

    let state_store = Arc::new(InMemoryStateStore::default());

    let database = Sqlite::new_in_dir(&settings.data.dir)?;

    let local_key_pair = derive_key_pair(&seed);
    let local_peer_id = PeerId::from(local_key_pair.clone().public());
    log::info!("Starting with peer_id: {}", local_peer_id);

    let transport = transport::build_comit_transport(local_key_pair);
    let behaviour = network::ComitNode::new(
        bitcoin_connector.clone(),
        ethereum_connector.clone(),
        Arc::clone(&state_store),
        seed,
        database.clone(),
        runtime.executor(),
    )?;

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id.clone());

    for addr in settings.network.listen.clone() {
        Swarm::listen_on(&mut swarm, addr).expect("Could not listen on specified address");
    }

    let swarm = Arc::new(Mutex::new(swarm));

    let deps = Facade {
        bitcoin_connector,
        ethereum_connector,
        state_store: Arc::clone(&state_store),
        seed,
        swarm: Arc::clone(&swarm),
        db: database.clone(),
        task_executor: runtime.executor(),
    };

    runtime.block_on(
        load_swaps::load_swaps_from_database(deps.clone())
            .boxed()
            .compat(),
    )?;

    spawn_warp_instance(&settings, local_peer_id, &mut runtime, deps);

    let swarm_worker = stream::poll_fn(move || swarm.lock().unwrap().poll())
        .for_each(|_| Ok(()))
        .map_err(|e| {
            log::error!("failed with {:?}", e);
        });

    runtime.spawn(swarm_worker);

    // Block the current thread.
    ::std::thread::park();
    Ok(())
}

fn derive_key_pair(seed: &Seed) -> identity::Keypair {
    let bytes = seed.sha256_with_seed(&[b"NODE_ID"]);
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
    identity::Keypair::Ed25519(key.into())
}

fn spawn_warp_instance<S: Network>(
    settings: &Settings,
    peer_id: PeerId,
    runtime: &mut tokio::runtime::Runtime,
    dependencies: Facade<S>,
) {
    let routes = route_factory::create(
        peer_id,
        dependencies,
        &settings.http_api.cors.allowed_origins,
    );

    let listen_addr = SocketAddr::new(
        settings.http_api.socket.address,
        settings.http_api.socket.port,
    );

    log::info!("Starting HTTP server on {:?}", listen_addr);

    let server = warp::serve(routes).bind(listen_addr);

    runtime.spawn(server);
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
