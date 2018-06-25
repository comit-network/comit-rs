#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
extern crate bitcoin;
extern crate bitcoin_rpc;
extern crate bitcoin_wallet;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_wallet;
extern crate exchange_service;
extern crate hex;
extern crate log;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate tiny_keccak;
extern crate uuid;
extern crate web3;

use bitcoin_wallet::PrivateKey;
use common_types::BitcoinQuantity;
use ethereum_wallet::InMemoryWallet;
use ethereum_wallet::ToEthereumAddress;
use exchange_service::bitcoin_fee_service::StaticBitcoinFeeService;
use exchange_service::ethereum_service::EthereumService;
use exchange_service::event_store::EventStore;
use exchange_service::gas_price_service::StaticGasPriceService;
use exchange_service::rocket_factory::create_rocket_instance;
use exchange_service::treasury_api_client::{DefaultApiClient, TreasuryApiUrl};
use hex::FromHex;
use secp256k1::SecretKey;
use std::env::var;
use std::str::FromStr;
use std::sync::Arc;
use web3::futures::Future;
use web3::types::Address as EthereumAddress;

fn var_or_exit(name: &str) -> String {
    match var(name) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("{} is not set but is required", name);
            std::process::exit(1);
        }
    }
}

// TODO: Make a nice command line interface here (using StructOpt f.e.)
fn main() {
    let _ = env_logger::init();
    let treasury_api_url = TreasuryApiUrl(var_or_exit("TREASURY_SERVICE_URL"));

    let api_client = DefaultApiClient {
        client: reqwest::Client::new(),
        url: treasury_api_url,
    };

    let event_store = EventStore::new();

    let private_key = var_or_exit("ETHEREUM_PRIVATE_KEY");
    let network_id = var_or_exit("ETHEREUM_NETWORK_ID");

    let private_key_data =
        <[u8; 32]>::from_hex(private_key).expect("Private key is not hex_encoded");

    let private_key = SecretKey::from_slice(&secp256k1::Secp256k1::new(), &private_key_data[..])
        .expect("Private key isn't valid");

    let network_id = u8::from_str(network_id.as_ref()).expect("Failed to parse network id");

    let wallet = InMemoryWallet::new(private_key_data, network_id)
        .expect("Failed to create wallet instance");

    let endpoint = var_or_exit("ETHEREUM_NODE_ENDPOINT");
    let gas_price = var("ETHEREUM_GAS_PRICE_IN_WEI")
        .map(|gas| u64::from_str(gas.as_str()).unwrap())
        .unwrap_or(2_000_000_000);

    let (_event_loop, transport) = web3::transports::Http::new(&endpoint).unwrap();

    let web3 = web3::api::Web3::new(transport);

    let address = private_key.to_ethereum_address();
    let nonce = web3.eth().transaction_count(address, None).wait().unwrap();
    println!("Nonce: {}", nonce);

    let ethereum_service = EthereumService::new(
        Arc::new(wallet),
        Arc::new(StaticGasPriceService::new(gas_price)),
        Arc::new(web3),
        nonce,
    );

    let exchange_refund_address =
        EthereumAddress::from_str(var_or_exit("EXCHANGE_REFUND_ADDRESS").as_str())
            .expect("EXCHANGE_REFUND_ADDRESS wasn't a valid ethereum address");

    let exchange_success_private_key =
        PrivateKey::from_str(var_or_exit("EXCHANGE_SUCCESS_PRIVATE_KEY").as_str()).unwrap();

    let bitcoin_rpc_client = {
        let url = var_or_exit("BITCOIN_RPC_URL");
        let username = var_or_exit("BITCOIN_RPC_USERNAME");
        let password = var_or_exit("BITCOIN_RPC_PASSWORD");

        bitcoin_rpc::BitcoinCoreClient::new(url.as_str(), username.as_str(), password.as_str())
    };

    let network = match var("BTC_NETWORK") {
        Ok(value) => match value.as_str() {
            "BTC_MAINNET" => bitcoin::network::constants::Network::Bitcoin,
            "BTC_TESTNET" => bitcoin::network::constants::Network::Testnet,
            "BTCORE_REGTEST" => bitcoin::network::constants::Network::BitcoinCoreRegtest,
            _ => panic!(
                "Please set environment variable BTC_NETWORK to one of the following values:\n\
                 - BTC_MAINNET\n- BTC_TESTNET\n- BTCORE_REGTEST"
            ),
        },
        Err(_) => bitcoin::network::constants::Network::BitcoinCoreRegtest,
    };

    let satoshi_per_kb = var_or_exit("BITCOIN_SATOSHI_PER_KB");
    let satoshi_per_kb =
        u64::from_str(&satoshi_per_kb).expect("Given value for rate cannot be parsed into u64");

    let rate_per_kb = BitcoinQuantity::from_satoshi(satoshi_per_kb);

    let bitcoin_fee_service = StaticBitcoinFeeService::new(rate_per_kb);

    create_rocket_instance(
        Arc::new(api_client),
        event_store,
        Arc::new(ethereum_service),
        Arc::new(bitcoin_rpc_client),
        exchange_refund_address,
        exchange_success_private_key,
        network,
        Arc::new(bitcoin_fee_service),
    ).launch();
}
