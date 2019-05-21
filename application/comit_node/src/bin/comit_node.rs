#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![deny(unsafe_code)]

use comit_node::{
    btsieve::BtsieveHttpClient,
    comit_client::Client,
    comit_i_routes,
    http_api::route_factory,
    load_settings::{load_settings, Opt},
    logging,
    network::{self, BamPeers},
    seed::Seed,
    settings::ComitNodeSettings,
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
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use structopt::StructOpt;

fn main() -> Result<(), failure::Error> {
    let opt = Opt::from_args();

    let settings = load_settings(opt)?;
    logging::set_up_logging(&settings);

    log::info!("Starting up with {:#?}", settings);

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
        Swarm::listen_on(&mut swarm, addr)?;
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

fn create_btsieve_api_client(settings: &ComitNodeSettings) -> BtsieveHttpClient {
    BtsieveHttpClient::new(
        &settings.btsieve.url,
        settings.btsieve.bitcoin.poll_interval_secs,
        settings.btsieve.bitcoin.network.as_str(),
        settings.btsieve.ethereum.poll_interval_secs,
        settings.btsieve.ethereum.network.as_str(),
    )
}

fn spawn_warp_instance<T: MetadataStore<SwapId>, S: StateStore, C: Client, BP: BamPeers>(
    settings: &ComitNodeSettings,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    protocol_dependencies: swap_protocols::alice::ProtocolDependencies<T, S, C>,
    get_bam_peers: Arc<BP>,
    peer_id: PeerId,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(
        metadata_store,
        state_store,
        protocol_dependencies,
        auth_origin(&settings),
        get_bam_peers,
        peer_id,
    );

    let listen_addr = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    log::info!("Starting HTTP server on {:?}", listen_addr);

    let server = warp::serve(routes).bind(listen_addr);

    runtime.spawn(server);
}

fn spawn_comit_i_instance(settings: ComitNodeSettings, runtime: &mut tokio::runtime::Runtime) {
    if let Some(comit_i_settings) = &settings.web_gui {
        let routes = comit_i_routes::create(settings.clone());

        let listen_addr = SocketAddr::new(comit_i_settings.address, comit_i_settings.port);

        log::info!("Starting comit-i HTTP server on {:?}", listen_addr);

        let server = warp::serve(routes).bind(listen_addr);

        runtime.spawn(server);
    }
}

fn auth_origin(settings: &ComitNodeSettings) -> String {
    let auth_origin = match &settings.web_gui {
        Some(http_socket) => format!("http://localhost:{}", http_socket.port),
        None => "http://localhost:3000".to_string(),
    };
    log::trace!("Auth origin enabled on: {}", auth_origin);
    auth_origin
}
