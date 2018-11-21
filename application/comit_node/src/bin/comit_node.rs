#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate ethereum_support;
#[macro_use]
extern crate log;
extern crate event_store;
extern crate futures;
extern crate tokio;
extern crate warp;

use bitcoin_rpc_client::*;
use bitcoin_support::Address as BitcoinAddress;

use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    comit_client,
    comit_server::ComitServer,
    ethereum_wallet::InMemoryWallet,
    gas_price_service::StaticGasPriceService,
    http_api::route_factory,
    key_store::KeyStore,
    ledger_query_service::DefaultLedgerQueryServiceApiClient,
    logging,
    settings::ComitNodeSettings,
    swap_protocols::{
        rfc003::{
            self,
            alice_ledger_actor::AliceLedgerActor,
            ledger_htlc_service::{BitcoinService, EthereumService},
            state_store::InMemoryStateStore,
        },
        InMemoryMetadataStore,
    },
    swaps::common::SwapId,
};

use comit_node::swap_protocols::rfc003::bob::PendingResponses;
use ethereum_support::*;
use event_store::InMemoryEventStore;
use futures::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};
use std::{env::var, marker::PhantomData, net::SocketAddr, sync::Arc, time::Duration};
use web3::{transports::Http, Web3};

// TODO: Make a nice command line interface here (using StructOpt f.e.) see #298
fn main() {
    logging::set_up_logging();
    let settings = load_settings();

    // TODO: Maybe not print settings because of private keys?
    info!("Starting up with {:#?}", settings);

    //TODO: Integrate all Ethereum keys in this keystore. See #185/#291
    let key_store = Arc::new(
        KeyStore::new(settings.bitcoin.extended_private_key)
            .expect("Could not HD derive keys from the private key"),
    );
    let event_store = Arc::new(InMemoryEventStore::default());
    let metadata_store = Arc::new(InMemoryMetadataStore::default());
    let state_store = Arc::new(InMemoryStateStore::default());
    let ethereum_service = create_ethereum_service(&settings);
    let bitcoin_service = create_bitcoin_service(&settings, &key_store);
    let ledger_query_service_api_client = create_ledger_query_service_api_client(&settings);
    let pending_responses = Arc::new(PendingResponses::default());

    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    let sender = spawn_alice_swap_request_handler_for_rfc003(
        &settings,
        Arc::clone(&event_store),
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&ethereum_service),
        Arc::clone(&bitcoin_service),
        Arc::clone(&ledger_query_service_api_client),
        Arc::clone(&key_store),
        &mut runtime,
    );

    spawn_warp_instance(
        &settings,
        Arc::clone(&event_store),
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&pending_responses),
        sender,
        &mut runtime,
    );

    let sender = spawn_bob_swap_request_handler_for_rfc003(
        Arc::clone(&event_store),
        Arc::clone(&metadata_store),
        Arc::clone(&state_store),
        Arc::clone(&pending_responses),
        Arc::clone(&ethereum_service),
        Arc::clone(&bitcoin_service),
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

fn create_ethereum_service(settings: &ComitNodeSettings) -> Arc<EthereumService> {
    let settings = &settings.ethereum;

    let ethereum_keypair = settings.private_key;

    let address = ethereum_keypair.public_key().to_ethereum_address();
    let wallet = InMemoryWallet::new(ethereum_keypair, settings.network_id);

    let (event_loop, transport) = Http::new(&settings.node_url.as_str()).unwrap();
    let web3 = Web3::new(transport);

    let nonce = web3.eth().transaction_count(address, None).wait().unwrap();
    info!(
        "ETH address derived from priv key: {}; AddressNonce: {}",
        address, nonce
    );

    Arc::new(EthereumService::new(
        Arc::new(wallet),
        Arc::new(StaticGasPriceService::new(settings.gas_price)),
        Arc::new((event_loop, web3)),
        nonce,
    ))
}

fn create_bitcoin_service(
    settings: &ComitNodeSettings,
    key_store: &KeyStore,
) -> Arc<BitcoinService> {
    let settings = &settings.bitcoin;

    //TODO: make it dynamically generated every X BTC. Could be done with #296
    let btc_bob_redeem_keypair = key_store.get_new_internal_keypair();
    let btc_bob_redeem_address = BitcoinAddress::p2wpkh(
        &btc_bob_redeem_keypair.public_key().into(),
        settings.network,
    );

    info!("btc_bob_redeem_address: {}", btc_bob_redeem_address);

    let bitcoin_rpc_client = Arc::new(bitcoin_rpc_client::BitcoinCoreClient::new(
        settings.node_url.as_str(),
        settings.node_username.as_str(),
        settings.node_password.as_str(),
    ));

    match bitcoin_rpc_client.get_blockchain_info() {
        Ok(blockchain_info) => {
            info!("Blockchain info:\n{:?}", blockchain_info);
            match bitcoin_rpc_client.validate_address(&btc_bob_redeem_address.clone()) {
                Ok(address_validation) => info!("Validation:\n{:?}", address_validation),
                Err(e) => error!("Could not validate BTC_BOB_REDEEM_ADDRESS: {:?}", e),
            };
        }
        Err(e) => error!("Could not connect to Bitcoin RPC because {:?}", e),
    };

    Arc::new(BitcoinService::new(
        bitcoin_rpc_client,
        settings.network,
        Arc::new(StaticBitcoinFeeService::new(settings.satoshi_per_byte)),
        btc_bob_redeem_address,
    ))
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
    event_store: Arc<InMemoryEventStore<SwapId>>,
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    pending_responses: Arc<PendingResponses<SwapId>>,
    sender: UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)>,
    runtime: &mut tokio::runtime::Runtime,
) {
    let routes = route_factory::create(
        event_store,
        metadata_store,
        state_store,
        pending_responses,
        sender,
    );

    let http_socket_address = SocketAddr::new(settings.http_api.address, settings.http_api.port);

    let server = warp::serve(routes).bind(http_socket_address);

    runtime.spawn(server);
}

fn spawn_alice_swap_request_handler_for_rfc003(
    settings: &ComitNodeSettings,
    event_store: Arc<InMemoryEventStore<SwapId>>,
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_service: Arc<BitcoinService>,
    ledger_query_service: Arc<DefaultLedgerQueryServiceApiClient>,
    key_store: Arc<KeyStore>,
    runtime: &mut tokio::runtime::Runtime,
) -> UnboundedSender<(SwapId, rfc003::alice::SwapRequestKind)> {
    let client_factory = Arc::new(comit_client::bam::BamClientPool::default());
    let comit_node_addr = settings.comit.remote_comit_node_url;

    let alice_actor = AliceLedgerActor::new(
        Arc::clone(&event_store),
        ledger_query_service,
        bitcoin_service,
        settings.bitcoin.network,
        ethereum_service,
        settings.ledger_query_service.bitcoin.poll_interval_secs,
        settings.ledger_query_service.ethereum.poll_interval_secs,
    );

    let (alice_actor_sender, alice_actor_future) = alice_actor.listen();
    runtime.spawn(alice_actor_future);

    let (sender, receiver) = mpsc::unbounded();

    let alice_swap_request_handler = rfc003::alice::SwapRequestHandler {
        receiver,
        metadata_store,
        key_store,
        state_store,
        client_factory,
        event_store,
        comit_node_addr,
        alice_actor_sender,
        phantom_data: PhantomData,
    };

    runtime.spawn(alice_swap_request_handler.start());

    sender
}

fn spawn_bob_swap_request_handler_for_rfc003(
    event_store: Arc<InMemoryEventStore<SwapId>>,
    metadata_store: Arc<InMemoryMetadataStore<SwapId>>,
    state_store: Arc<InMemoryStateStore<SwapId>>,
    pending_responses: Arc<PendingResponses<SwapId>>,
    ethereum_service: Arc<EthereumService>,
    bitcoin_service: Arc<BitcoinService>,
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
        event_store,
        state_store,
        lqs_api_client,
        key_store,
        ethereum_service,
        bitcoin_service,
        bitcoin_poll_interval,
        ethereum_poll_interval,
        pending_responses,
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
