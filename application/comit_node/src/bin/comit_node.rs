#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;
extern crate comit_node;
extern crate common_types;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate hex;
#[macro_use]
extern crate log;
extern crate event_store;
extern crate logging;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_json;
extern crate tiny_keccak;
extern crate tokio;
extern crate uuid;
extern crate web3;

use bitcoin_rpc_client::BitcoinRpcApi;
use bitcoin_support::{Network, PrivateKey};
use comit_node::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    comit_node_api_client::DefaultApiClient as ComitNodeClient,
    comit_server::ComitServer,
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    swap_protocols::rfc003::ledger_htlc_service::{BitcoinService, EthereumService},
};
use ethereum_support::*;
use ethereum_wallet::InMemoryWallet;
use event_store::InMemoryEventStore;
use hex::FromHex;
use secp256k1_support::KeyPair;
use std::{env::var, net::SocketAddr, str::FromStr, sync::Arc};
use web3::{transports::Http, Web3};

// TODO: Make a nice command line interface here (using StructOpt f.e.)
fn main() {
    logging::set_up_logging();

    let event_store = Arc::new(InMemoryEventStore::new());
    let rocket_event_store = event_store.clone();
    let comit_server_event_store = event_store.clone();

    let network_id = var_or_exit("ETHEREUM_NETWORK_ID");

    let network_id = u8::from_str(network_id.as_ref()).expect("Failed to parse network id");

    let secret_key_hex = var_or_exit("ETHEREUM_PRIVATE_KEY");

    let secret_key_data =
        <[u8; 32]>::from_hex(secret_key_hex).expect("Private key is not hex_encoded");

    let eth_keypair: KeyPair =
        KeyPair::from_secret_key_slice(&secret_key_data).expect("Private key isn't valid");

    let address = eth_keypair.public_key().to_ethereum_address();
    let wallet = InMemoryWallet::new(eth_keypair, network_id);

    let endpoint = var_or_exit("ETHEREUM_NODE_ENDPOINT");

    let (event_loop, transport) = Http::new(&endpoint).unwrap();
    let web3 = Web3::new(transport);

    let gas_price = var("ETHEREUM_GAS_PRICE_IN_WEI")
        .map(|gas| u64::from_str(gas.as_str()).unwrap())
        .unwrap_or(2_000_000_000);
    info!("set ETHEREUM_GAS_PRICE_IN_WEI={}", gas_price);

    let nonce = web3.eth().transaction_count(address, None).wait().unwrap();
    info!(
        "ETH address derived from priv key: {}; AddressNonce: {}",
        address, nonce
    );

    let ethereum_service = EthereumService::new(
        Arc::new(wallet),
        Arc::new(StaticGasPriceService::new(gas_price)),
        Arc::new((event_loop, web3)),
        nonce,
    );

    let bob_refund_address = ethereum_support::Address::from_str(
        var_or_exit("BOB_REFUND_ADDRESS").as_str(),
    ).expect("BOB_REFUND_ADDRESS wasn't a valid ethereum address");

    let bob_success_private_key =
        PrivateKey::from_str(var_or_exit("BTC_BOB_SUCCESS_PRIVATE_KEY").as_str()).unwrap();
    let bob_success_keypair: KeyPair = bob_success_private_key.secret_key().clone().into();

    let btc_bob_redeem_address = bitcoin_support::Address::from_str(
        var_or_exit("BTC_BOB_REDEEM_ADDRESS").as_str(),
    ).expect("BTC Bob Redeem Address is Invalid");

    let bitcoin_rpc_client = {
        let url = var_or_exit("BITCOIN_RPC_URL");
        let username = var_or_exit("BITCOIN_RPC_USERNAME");
        let password = var_or_exit("BITCOIN_RPC_PASSWORD");

        bitcoin_rpc_client::BitcoinCoreClient::new(
            url.as_str(),
            username.as_str(),
            password.as_str(),
        )
    };

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

    let network = match var("BTC_NETWORK") {
        Ok(value) => match value.as_str() {
            "BTC_MAINNET" => Network::Bitcoin,
            "BTC_TESTNET" => Network::Testnet,
            "BTCORE_REGTEST" => Network::Regtest,
            _ => panic!(
                "Please set environment variable BTC_NETWORK to one of the following values:\n\
                 - BTC_MAINNET\n- BTC_TESTNET\n- BTCORE_REGTEST"
            ),
        },
        Err(_) => Network::Regtest,
    };
    info!("set BTC_NETWORK={}", network);

    let satoshi_per_kb = var_or_exit("BITCOIN_SATOSHI_PER_KB");
    let satoshi_per_kb =
        f64::from_str(&satoshi_per_kb).expect("Given value for rate cannot be parsed into f64");
    let bitcoin_fee_service = StaticBitcoinFeeService::new(satoshi_per_kb);
    let bitcoin_rpc_client = Arc::new(bitcoin_rpc_client);
    let bitcoin_fee_service = Arc::new(bitcoin_fee_service);
    let bitcoin_service = BitcoinService::new(
        bitcoin_rpc_client.clone(),
        network,
        bitcoin_fee_service.clone(),
        btc_bob_redeem_address.clone(),
    );

    let comit_node_socket_addr = {
        let socket_addr = var_or_exit("COMIT_NODE_SOCKET_ADDR");
        SocketAddr::from_str(&socket_addr).unwrap()
    };

    let comit_listen_port = var_or_exit("COMIT_PORT");

    {
        let bob_refund_address = bob_refund_address.clone();
        let bob_success_keypair = bob_success_keypair.clone();
        let network = network.clone();

        std::thread::spawn(move || {
            create_rocket_instance(
                rocket_event_store,
                Arc::new(ethereum_service),
                Arc::new(bitcoin_service),
                bob_refund_address,
                bob_success_keypair,
                network,
                Arc::new(ComitNodeClient::new(comit_node_socket_addr)),
            ).launch();
        });
    }

    let server = ComitServer::new(
        comit_server_event_store,
        bob_refund_address,
        bob_success_keypair,
    );

    tokio::run(
        server
            .listen(format!("0.0.0.0:{}", comit_listen_port).parse().unwrap())
            .map_err(|e| {
                error!("ComitServer shutdown: {:?}", e);
            }),
    );
}

fn var_or_exit(name: &str) -> String {
    match var(name) {
        Ok(value) => {
            info!("Set {}={}", name, value);
            value
        }
        Err(_) => {
            eprintln!("{} is not set but is required", name);
            std::process::exit(1);
        }
    }
}
