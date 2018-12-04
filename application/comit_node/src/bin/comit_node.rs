#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]
extern crate comit_node;
extern crate ethereum_support;
#[macro_use]
extern crate log;
extern crate futures;
extern crate tokio;
extern crate warp;

use comit_node::{
    comit_client,
    comit_server::ComitServer,
    http_api::route_factory,
    key_store::KeyStore,
    ledger_query_service::DefaultLedgerQueryServiceApiClient,
    logging,
    settings::ComitNodeSettings,
    swap_protocols::{
        rfc003::{self, state_store::InMemoryStateStore},
        InMemoryMetadataStore,
    },
    swaps::common::SwapId,
};

use ethereum_support::*;
use futures::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};
use std::{env::var, marker::PhantomData, net::SocketAddr, sync::Arc, time::Duration};

// TODO: Make a nice command line interface here (using StructOpt f.e.) see #298
fn main() {
    logging::set_up_logging();
    let settings = load_settings();

    // TODO: Maybe not print settings because of private keys?
    info!("Starting up with {:#?}", settings);

    let key_store = Arc::new(
        KeyStore::new(settings.bitcoin.extended_private_key)
            .expect("Could not HD derive keys from the private key"),
    );

    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());
    let ledger_query_service_api_client = create_ledger_query_service_api_client(&settings);

    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let sender = spawn_alice_swap_request_handler_for_rfc003(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&key_store),
        Arc::clone(&ledger_query_service_api_client),
        settings.ledger_query_service.bitcoin.poll_interval_secs,
        settings.ledger_query_service.ethereum.poll_interval_secs,
        &mut runtime,
    );

    spawn_warp_instance(
        &settings,
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        sender,
        Arc::clone(&key_store),
        &mut runtime,
    );

    let sender = spawn_bob_swap_request_handler_for_rfc003(
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&ledger_query_service_api_client),
        Arc::clone(&key_store),
        settings.ledger_query_service.bitcoin.poll_interval_secs,
        settings.ledger_query_service.ethereum.poll_interval_secs,
        &mut runtime,
    );

    spawn_comit_server(&settings, sender, &mut runtime);

    // Block the current thread.
    ::std::thread::park();
}

fn load_settings() -> ComitNodeSettings {
    let comit_config_path = var_or_default("COMIT_NODE_CONFIG_PATH", "~/.config/comit_node".into());
    let run_mode_config = var_or_default("RUN_MODE", "development".into());
    let default_config = format!("{}/{}", comit_config_path.trim(), "default");
    let erc20_config = format!("{}/{}", comit_config_path.trim(), "erc20");
    let run_mode_config = format!("{}/{}", comit_config_path.trim(), run_mode_config);

    let settings = ComitNodeSettings::new(default_config, run_mode_config, erc20_config);
    settings.unwrap()
}

fn create_ledger_query_service_api_client(
    settings: &ComitNodeSettings,
) -> Arc<DefaultLedgerQueryServiceApiClient> {
    Arc::new(DefaultLedgerQueryServiceApiClient::new(
        &settings.ledger_query_service.url,
    ))
}

fn spawn_warp_instance(
    settings: &ComitNodeSettings,
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    sender: UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)>,
    key_store: Arc<KeyStore>,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(metadata_store, state_store, sender, key_store);

    let http_socket_address = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    let server = warp::serve(routes).bind(http_socket_address);

    runtime.spawn(server);
}

fn spawn_alice_swap_request_handler_for_rfc003(
    settings: &ComitNodeSettings,
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    key_store: Arc<KeyStore>,
    lqs_api_client: Arc<DefaultLedgerQueryServiceApiClient>,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
    runtime: &mut tokio::runtime::Runtime,
) -> UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)> {
    let client_factory = Arc::new(comit_client::bam::BamClientPool::default());
    let comit_node_addr = settings.comit.remote_comit_node_url;

    let (sender, receiver) = mpsc::unbounded();

    let alice_swap_request_handler = rfc003::alice::SwapRequestHandler {
        receiver,
        metadata_store,
        key_store,
        state_store,
        client_factory,
        comit_node_addr,
        bitcoin_poll_interval,
        ethereum_poll_interval,
        lqs_api_client,
        phantom_data: PhantomData,
    };

    runtime.spawn(alice_swap_request_handler.start());

    sender
}

fn spawn_bob_swap_request_handler_for_rfc003(
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    lqs_api_client: Arc<DefaultLedgerQueryServiceApiClient>,
    key_store: Arc<KeyStore>,
    bitcoin_poll_interval: Duration,
    ethereum_poll_interval: Duration,
    runtime: &mut tokio::runtime::Runtime,
) -> UnboundedSender<(
    SwapId,
    rfc003::bob::SwapRequestKind,
    oneshot::Sender<rfc003::bob::SwapResponseKind>,
)> {
    let (sender, receiver) = mpsc::unbounded();

    let bob_swap_request_handler = rfc003::bob::SwapRequestHandler {
        receiver,
        metadata_store,
        state_store,
        lqs_api_client,
        key_store,
        bitcoin_poll_interval,
        ethereum_poll_interval,
    };

    runtime.spawn(bob_swap_request_handler.start());

    sender
}

fn spawn_comit_server(
    settings: &ComitNodeSettings,
    sender: UnboundedSender<(
        SwapId,
        rfc003::bob::SwapRequestKind,
        oneshot::Sender<rfc003::bob::SwapResponseKind>,
    )>,

    runtime: &mut tokio::runtime::Runtime,
) {
    let server = ComitServer::new(sender);

    runtime.spawn(server.listen(settings.comit.comit_listen).map_err(|e| {
        error!("ComitServer shutdown: {:?}", e);
    }));
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
