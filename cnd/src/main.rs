#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]
use btsieve::{bitcoin::BitcoindConnector, ethereum::Web3Connector};
use cnd::{
    config::{self, Settings},
    http_api::{self, route_factory},
    network::{self, Network, SendRequest},
    seed::{Seed, SwapSeed},
    swap_protocols::{
        rfc003::{
            state_store::{InMemoryStateStore, StateStore},
            Spawn,
        },
        InMemoryMetadataStore, LedgerConnectors, MetadataStore,
    },
};
use futures::{stream, Future, Stream};
use libp2p::{
    identity::{self, ed25519},
    PeerId, Swarm,
};
use rand::rngs::OsRng;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use structopt::StructOpt;

mod cli;
mod logging;

fn main() -> Result<(), failure::Error> {
    let options = cli::Options::from_args();

    let config_file = options
        .config_file
        .map(config::File::read)
        .unwrap_or_else(config::File::read_or_create_default)?;
    let settings = Settings::from_config_file_and_defaults(config_file);

    let base_log_level = settings.logging.level;
    logging::initialize(base_log_level, settings.logging.structured)?;

    let seed = match options.seed_file {
        Some(file) => Seed::from_file(file)?,
        None => Seed::from_default_file_or_generate(OsRng)?,
    };

    let mut runtime = tokio::runtime::Runtime::new()?;

    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());

    let bitcoin_connector = {
        let config::file::Bitcoin { node_url, network } = settings.clone().bitcoin;
        BitcoindConnector::new(node_url, network)?
    };

    let (ethereum_connector, _event_loop_handle) =
        { Web3Connector::new(settings.clone().ethereum.node_url)? };

    let ledger_events = LedgerConnectors {
        bitcoin_connector,
        ethereum_connector,
    };

    let local_key_pair = derive_key_pair(&seed);
    let local_peer_id = PeerId::from(local_key_pair.clone().public());
    log::info!("Starting with peer_id: {}", local_peer_id);

    let transport = libp2p::build_development_transport(local_key_pair);
    let behaviour = network::ComitNode::new(
        ledger_events.clone(),
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        seed,
    )?;

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id.clone());

    for addr in settings.network.listen.clone() {
        Swarm::listen_on(&mut swarm, addr).expect("Could not listen on specified address");
    }

    let swarm = Arc::new(Mutex::new(swarm));

    let http_api_dependencies = http_api::Dependencies {
        ledger_events: ledger_events.clone(),
        metadata_store: Arc::clone(&metadata_store),
        state_store: Arc::clone(&state_store),
        seed,
        swarm: Arc::clone(&swarm),
    };

    spawn_warp_instance(
        &settings,
        local_peer_id,
        &mut runtime,
        http_api_dependencies,
    );

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

fn spawn_warp_instance<
    D: Clone + MetadataStore + StateStore + Network + SendRequest + Spawn + SwapSeed,
>(
    settings: &Settings,
    peer_id: PeerId,
    runtime: &mut tokio::runtime::Runtime,
    dependencies: D,
) {
    let routes = route_factory::create(peer_id, dependencies);

    let listen_addr = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    log::info!("Starting HTTP server on {:?}", listen_addr);

    let server = warp::serve(routes).bind(listen_addr);

    runtime.spawn(server);
}
