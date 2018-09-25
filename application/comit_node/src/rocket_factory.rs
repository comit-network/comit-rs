use bitcoin_support::Network;
use comit_node_api_client::ApiClient;
use ethereum_support;
use event_store::InMemoryEventStore;
use rand::OsRng;
use rocket::{
    self,
    config::{Config, Environment},
    Rocket,
};
use secp256k1_support::KeyPair;
use std::sync::{Arc, Mutex};
use swap_protocols::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003::ledger_htlc_service::{BitcoinHtlcParams, EtherHtlcParams, LedgerHtlcService},
};
use swaps::{common::TradeId, eth_btc};

pub fn create_rocket_instance(
    event_store: Arc<InMemoryEventStore<TradeId>>,
    ethereum_service: Arc<LedgerHtlcService<Ethereum, EtherHtlcParams>>,
    bitcoin_service: Arc<LedgerHtlcService<Bitcoin, BitcoinHtlcParams>>,
    bob_refund_address: ethereum_support::Address,
    bob_success_keypair: KeyPair,
    network: Network,
    bob_client: Arc<ApiClient>,
    address: String,
    port: u16,
    logging: bool,
) -> rocket::Rocket {
    let rng = OsRng::new().expect("Failed to get randomness from OS");

    try_config(address, port, logging)
        .mount(
            "/cli/", //Endpoints for interaction with the CLI
            //todo come up with a better name
            routes![
                eth_btc::cli::buy_routes::get_redeem_orders,
                eth_btc::cli::buy_routes::post_buy_offers,
                eth_btc::cli::buy_routes::post_buy_orders,
                eth_btc::cli::sell_routes::post_sell_offers,
                eth_btc::cli::sell_routes::post_sell_orders,
            ],
        ).mount(
            "/ledger/", //Endpoints for notifying about ledger events
            routes![
                // TODO will be removed once we have the Ledger Query Service
                eth_btc::ledger::buy_routes::post_contract_deployed,
                eth_btc::ledger::buy_routes::post_orders_funding,
                eth_btc::ledger::buy_routes::post_revealed_secret,
                eth_btc::ledger::sell_routes::post_orders_funding,
                eth_btc::ledger::sell_routes::post_revealed_secret,
            ],
        ).manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_service)
        .manage(bob_success_keypair)
        .manage(bob_refund_address)
        .manage(network)
        .manage(bob_client)
        .manage(Mutex::new(rng))
}

fn try_config(address: String, port: u16, logging: bool) -> Rocket {
    //TODO change environment?
    let config = Config::build(Environment::Development)
        .address(address.clone())
        .port(port)
        .finalize();
    match config {
        Ok(config) => rocket::custom(config, logging),
        Err(error) => {
            error!("{:?}", error);
            error!(
                "Could not start rocket with {}:{}, falling back to default",
                address, port
            );
            rocket::ignite()
        }
    }
}
