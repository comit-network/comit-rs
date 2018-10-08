#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate comit_wallet;
extern crate ethereum_support;
extern crate ethereum_wallet;
#[macro_use]
extern crate log;
extern crate event_store;
extern crate gotham;
extern crate logging;
extern crate tokio;
extern crate web3;

use bitcoin_rpc_client::BitcoinRpcApi;
use bitcoin_support::Address as BitcoinAddress;

use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    comit_client,
    comit_server::ComitServer,
    gas_price_service::StaticGasPriceService,
    gotham_factory,
    ledger_query_service::DefaultLedgerQueryServiceApiClient,
    rocket_factory::create_rocket_instance,
    settings::ComitNodeSettings,
    swap_protocols::rfc003::ledger_htlc_service::{BitcoinService, EthereumService},
};
use comit_wallet::KeyStore;
use ethereum_support::*;
use ethereum_wallet::InMemoryWallet;
use event_store::InMemoryEventStore;
use std::{env::var, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use web3::{transports::Http, Web3};

// TODO: Make a nice command line interface here (using StructOpt f.e.) see #298
fn main() {
    logging::set_up_logging();
    let settings = load_settings();

    // TODO: Maybe not print settings because of private keys?
    info!("Starting up with {:#?}", settings);

    let event_store = Arc::new(InMemoryEventStore::default());
    let rocket_event_store = event_store.clone();
    let comit_server_event_store = event_store.clone();
    let gotham_event_store = event_store.clone();

    let eth_keypair = settings.ethereum.private_key;

    let address = eth_keypair.public_key().to_ethereum_address();
    let wallet = InMemoryWallet::new(eth_keypair, settings.ethereum.network_id);

    let (event_loop, transport) = Http::new(&settings.ethereum.node_url.as_str()).unwrap();
    let web3 = Web3::new(transport);

    let nonce = web3.eth().transaction_count(address, None).wait().unwrap();
    info!(
        "ETH address derived from priv key: {}; AddressNonce: {}",
        address, nonce
    );

    let ethereum_service = Arc::new(EthereumService::new(
        Arc::new(wallet),
        Arc::new(StaticGasPriceService::new(settings.ethereum.gas_price)),
        Arc::new((event_loop, web3)),
        nonce,
    ));

    let _eth_refund_address = settings.swap.eth_refund_address;

    let btc_network = settings.bitcoin.network;

    //TODO: Integrate all Ethereum keys in this keystore. See #185/#291
    let key_store = Arc::new(
        KeyStore::new(settings.bitcoin.extended_private_key)
            .expect("Could not HD derive keys from the private key"),
    );

    //TODO: make it dynamically generated every X BTC. Could be done with #296
    let btc_bob_redeem_keypair = key_store.get_new_internal_keypair();
    let btc_bob_redeem_address =
        BitcoinAddress::p2wpkh(btc_bob_redeem_keypair.public_key(), btc_network);

    info!("btc_bob_redeem_address: {}", btc_bob_redeem_address);

    let bitcoin_rpc_client = Arc::new(bitcoin_rpc_client::BitcoinCoreClient::new(
        settings.bitcoin.node_url.as_str(),
        settings.bitcoin.node_username.as_str(),
        settings.bitcoin.node_password.as_str(),
    ));

    match bitcoin_rpc_client.get_blockchain_info() {
        Ok(blockchain_info) => {
            info!("Blockchain info:\n{:?}", blockchain_info);
            match bitcoin_rpc_client.validate_address(&bitcoin_rpc_client::Address::from(
                btc_bob_redeem_address.clone(),
            )) {
                Ok(address_validation) => info!("Validation:\n{:?}", address_validation),
                Err(e) => error!("Could not validate BTC_BOB_REDEEM_ADDRESS: {}", e),
            };
        }
        Err(e) => error!("Could not connect to Bitcoin RPC:\n{}", e),
    };

    let satoshi_per_kb = settings.bitcoin.satoshi_per_byte;
    let bitcoin_fee_service = StaticBitcoinFeeService::new(satoshi_per_kb);
    let bitcoin_fee_service = Arc::new(bitcoin_fee_service);
    let bitcoin_service = Arc::new(BitcoinService::new(
        bitcoin_rpc_client.clone(),
        settings.bitcoin.network,
        bitcoin_fee_service.clone(),
        btc_bob_redeem_address.clone(),
    ));

    {
        let http_api_address_gotham = settings.http_api.address.clone();
        let http_api_address_rocket = settings.http_api.address.clone();
        let http_api_port = settings.http_api.port;
        let http_api_logging = settings.http_api.logging;
        let remote_comit_node_url = settings.comit.remote_comit_node_url;
        let key_store_rocket = key_store.clone();
        let ethereum_service = ethereum_service.clone();
        let bitcoin_service = bitcoin_service.clone();

        let client_pool = comit_client::bam::BamClientPool::default();

        let gotham_router = gotham_factory::create_gotham_router(
            gotham_event_store,
            Arc::new(client_pool),
            remote_comit_node_url,
            key_store.clone(),
        );

        std::thread::spawn(move || {
            gotham::start(
                SocketAddr::from_str(
                    format!("{}:{}", http_api_address_gotham, http_api_port).as_str(),
                ).unwrap(),
                gotham_router,
            );
        });

        std::thread::spawn(move || {
            create_rocket_instance(
                rocket_event_store,
                ethereum_service,
                bitcoin_service,
                key_store_rocket,
                btc_network,
                http_api_address_rocket,
                http_api_port + 2,
                http_api_logging,
            ).launch();
        });
    }

    let ledger_query_service = Arc::new(DefaultLedgerQueryServiceApiClient::new(
        settings.bitcoin.lqs_url,
    ));

    let server = ComitServer::new(
        comit_server_event_store,
        key_store.clone(),
        ethereum_service.clone(),
        bitcoin_service.clone(),
        ledger_query_service,
        btc_network,
        Duration::from_secs(settings.bitcoin.queries_poll_interval_secs),
    );

    tokio::run(server.listen(settings.comit.comit_listen).map_err(|e| {
        error!("ComitServer shutdown: {:?}", e);
    }));
}

fn load_settings() -> ComitNodeSettings {
    let comit_config_path = var_or_default("COMIT_NODE_CONFIG_PATH", "~/.config/comit_node".into());
    let run_mode_config = var_or_default("RUN_MODE", "development".into());
    let default_config = format!("{}/{}", comit_config_path.trim(), "default");
    let run_mode_config = format!("{}/{}", comit_config_path.trim(), run_mode_config);

    let settings = ComitNodeSettings::new(default_config, run_mode_config);
    settings.unwrap()
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
