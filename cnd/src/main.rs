#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use cnd::{
    btsieve::BtsieveHttpClient,
    comit_client::Client,
    comit_i_routes,
    config::{self, Settings},
    http_api::route_factory,
    network::{self, SwarmInfo},
    seed::Seed,
    swap_protocols::{
        self,
        metadata_store::MetadataStore,
        rfc003::state_store::{InMemoryStateStore, StateStore},
        InMemoryMetadataStore, SwapId,
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
        .unwrap_or_else(|| {
            config::File::read_or_create_default(
                directories::UserDirs::new()
                    .as_ref()
                    .map(|dirs| dirs.home_dir()),
                OsRng,
            )
        })?;
    let settings = Settings::from_config_file_and_defaults(config_file);

    let base_log_level = settings.logging.level;
    logging::initialize(base_log_level, settings.logging.structured)?;

    let mut runtime = tokio::runtime::Runtime::new()?;

    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());
    let btsieve_client = create_btsieve_api_client(&settings);

    let bob_protocol_dependencies = swap_protocols::bob::ProtocolDependencies {
        ledger_events: btsieve_client.clone().into(),
        metadata_store: Arc::clone(&metadata_store),
        state_store: Arc::clone(&state_store),
        seed: settings.comit.secret_seed,
    };

    let local_key_pair = derive_key_pair(&settings.comit.secret_seed);
    let local_peer_id = PeerId::from(local_key_pair.clone().public());
    log::info!("Starting with peer_id: {}", local_peer_id);

    let transport = libp2p::build_development_transport(local_key_pair);
    let behaviour = network::Behaviour::new(bob_protocol_dependencies, runtime.executor())?;

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id.clone());

    for addr in settings.network.listen.clone() {
        // FIXME: Replace `expect` with `?`
        // This can be solved by building our own Transport instead of using
        // `build_development_transport`
        Swarm::listen_on(&mut swarm, addr).expect("Could not listen on specified address");
    }

    let swarm = Arc::new(Mutex::new(swarm));

    let alice_protocol_dependencies = swap_protocols::alice::ProtocolDependencies {
        ledger_events: btsieve_client.into(),
        metadata_store: Arc::clone(&metadata_store),
        state_store: Arc::clone(&state_store),
        seed: settings.comit.secret_seed,
        client: Arc::clone(&swarm),
    };

    spawn_warp_instance(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        alice_protocol_dependencies,
        Arc::clone(&swarm),
        local_peer_id,
        &mut runtime,
    );

    spawn_comit_i_instance(settings, &mut runtime);

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

fn derive_key_pair(secret_seed: &Seed) -> identity::Keypair {
    let bytes = secret_seed.sha256_with_seed(&[b"NODE_ID"]);
    let key = ed25519::SecretKey::from_bytes(bytes).expect("we always pass 32 bytes");
    identity::Keypair::Ed25519(key.into())
}

fn create_btsieve_api_client(settings: &Settings) -> BtsieveHttpClient {
    BtsieveHttpClient::new(
        &settings.btsieve.url,
        settings.btsieve.bitcoin.poll_interval_secs,
        settings.btsieve.bitcoin.network,
        settings.btsieve.ethereum.poll_interval_secs,
        settings.btsieve.ethereum.network,
    )
}

fn spawn_warp_instance<T: MetadataStore<SwapId>, S: StateStore, C: Client, SI: SwarmInfo>(
    settings: &Settings,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    protocol_dependencies: swap_protocols::alice::ProtocolDependencies<T, S, C>,
    swarm_info: Arc<SI>,
    peer_id: PeerId,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(
        metadata_store,
        state_store,
        protocol_dependencies,
        auth_origin(&settings),
        swarm_info,
        peer_id,
    );

    let listen_addr = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    log::info!("Starting HTTP server on {:?}", listen_addr);

    let server = warp::serve(routes).bind(listen_addr);

    runtime.spawn(server);
}

fn spawn_comit_i_instance(settings: Settings, runtime: &mut tokio::runtime::Runtime) {
    if let Some(comit_i_settings) = &settings.web_gui {
        let routes = comit_i_routes::create(settings.clone());

        let listen_addr = SocketAddr::new(comit_i_settings.address, comit_i_settings.port);

        log::info!("Starting comit-i HTTP server on {:?}", listen_addr);

        let server = warp::serve(routes).bind(listen_addr);

        runtime.spawn(server);
    }
}

fn auth_origin(settings: &Settings) -> String {
    let auth_origin = match &settings.web_gui {
        Some(http_socket) => format!("http://localhost:{}", http_socket.port),
        None => "http://localhost:3000".to_string(),
    };
    log::trace!("Auth origin enabled on: {}", auth_origin);
    auth_origin
}
