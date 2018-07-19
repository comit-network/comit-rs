#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
extern crate bitcoin_rpc;
extern crate bitcoin_support;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_support;
extern crate ethereum_wallet;
extern crate exchange_service;
extern crate hex;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
extern crate secp256k1_support;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate tiny_keccak;
extern crate uuid;

use bitcoin_rpc::BitcoinRpcApi;
use bitcoin_support::{Network, PrivateKey};
use ethereum_support::*;
use ethereum_wallet::InMemoryWallet;
use exchange_service::{
    bitcoin_fee_service::StaticBitcoinFeeService,
    ethereum_service::EthereumService,
    event_store::EventStore,
    gas_price_service::StaticGasPriceService,
    rocket_factory::create_rocket_instance,
    treasury_api_client::{DefaultApiClient, TreasuryApiUrl},
};
use hex::FromHex;
use secp256k1_support::KeyPair;
use std::{env::var, str::FromStr, sync::Arc};

// TODO: Make a nice command line interface here (using StructOpt f.e.)
fn main() {
    let _ = env_logger::init();
    let treasury_api_url = TreasuryApiUrl(var_or_exit("TREASURY_SERVICE_URL"));
    info!("set TREASURY_SERVICE_URL={:?}", treasury_api_url);

    let api_client = DefaultApiClient {
        client: reqwest::Client::new(),
        url: treasury_api_url,
    };

    let event_store = EventStore::new();

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

    let web3 = Web3Client::new(endpoint);

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
        Arc::new(web3),
        nonce,
    );

    let exchange_refund_address =
        ethereum_support::Address::from_str(var_or_exit("EXCHANGE_REFUND_ADDRESS").as_str())
            .expect("EXCHANGE_REFUND_ADDRESS wasn't a valid ethereum address");

    let exchange_success_private_key =
        PrivateKey::from_str(var_or_exit("EXCHANGE_SUCCESS_PRIVATE_KEY").as_str()).unwrap();
    let exchange_success_keypair: KeyPair =
        exchange_success_private_key.secret_key().clone().into();

    let btc_exchange_redeem_address = bitcoin_support::Address::from_str(
        var_or_exit("BTC_EXCHANGE_REDEEM_ADDRESS").as_str(),
    ).expect("BTC Exchange Redeem Address is Invalid");

    let bitcoin_rpc_client = {
        let url = var_or_exit("BITCOIN_RPC_URL");
        let username = var_or_exit("BITCOIN_RPC_USERNAME");
        let password = var_or_exit("BITCOIN_RPC_PASSWORD");

        bitcoin_rpc::BitcoinCoreClient::new(url.as_str(), username.as_str(), password.as_str())
    };

    match bitcoin_rpc_client.get_blockchain_info() {
        Ok(blockchain_info) => {
            info!("Blockchain info:\n{:?}", blockchain_info);
            match bitcoin_rpc_client.validate_address(&bitcoin_rpc::Address::from(
                btc_exchange_redeem_address.clone(),
            )) {
                Ok(address_validation) => info!("Validation:\n{:?}", address_validation),
                Err(e) => error!("Could not validate BTC_EXCHANGE_REDEEM_ADDRESS: {}", e),
            };
        }
        Err(e) => error!("Could not connect to Bitcoin RPC:\n{}", e),
    };

    let network = match var("BTC_NETWORK") {
        Ok(value) => match value.as_str() {
            "BTC_MAINNET" => Network::Bitcoin,
            "BTC_TESTNET" => Network::Testnet,
            "BTCORE_REGTEST" => Network::BitcoinCoreRegtest,
            _ => panic!(
                "Please set environment variable BTC_NETWORK to one of the following values:\n\
                 - BTC_MAINNET\n- BTC_TESTNET\n- BTCORE_REGTEST"
            ),
        },
        Err(_) => Network::BitcoinCoreRegtest,
    };
    info!("set BTC_NETWORK={}", network);

    let satoshi_per_kb = var_or_exit("BITCOIN_SATOSHI_PER_KB");
    let satoshi_per_kb =
        f64::from_str(&satoshi_per_kb).expect("Given value for rate cannot be parsed into f64");
    let bitcoin_fee_service = StaticBitcoinFeeService::new(satoshi_per_kb);

    create_rocket_instance(
        Arc::new(api_client),
        event_store,
        Arc::new(ethereum_service),
        Arc::new(bitcoin_rpc_client),
        exchange_refund_address,
        exchange_success_keypair,
        btc_exchange_redeem_address,
        network,
        Arc::new(bitcoin_fee_service),
    ).launch();
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
