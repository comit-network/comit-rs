use bitcoin_support::Network;
use comit_node_api_client::ApiClient;
use common_types::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use ethereum_support;
use event_store::InMemoryEventStore;
use ledger_htlc_service::LedgerHtlcService;
use rand::OsRng;
use rocket;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use std::sync::Mutex; //TODO rename
use swaps::{common::TradeId, eth_btc};

pub fn create_rocket_instance(
    event_store: InMemoryEventStore<TradeId>,
    ethereum_service: Arc<LedgerHtlcService<Ethereum>>,
    bitcoin_service: Arc<LedgerHtlcService<Bitcoin>>,
    bob_refund_address: ethereum_support::Address,
    bob_success_keypair: KeyPair,
    network: Network,
    bob_client: Arc<ApiClient>,
) -> rocket::Rocket {
    let rng = OsRng::new().expect("Failed to get randomness from OS");

    rocket::ignite()
        .mount(
            "/", //Endpoints for inbetween the nodes
            routes![
                eth_btc::buy::routes::post_buy_orders,
                eth_btc::sell::routes::post_orders_funding,
                eth_btc::sell::routes::post_revealed_secret,
            ],
        )
        .mount(
            "/cli/", //Endpoints for interaction with the CLI
            //todo come up with a better name
            routes![
                eth_btc::cli::buy_routes::get_redeem_orders,
                eth_btc::cli::buy_routes::post_buy_offers,
                eth_btc::cli::buy_routes::post_buy_orders,
                eth_btc::cli::sell_routes::post_sell_offers,
                eth_btc::cli::sell_routes::post_sell_orders,
                eth_btc::ledger::buy_routes::post_contract_deployed, // TODO move into own route when cli working
                eth_btc::ledger::buy_routes::post_orders_funding, // TODO move into own route when cli working
                eth_btc::ledger::buy_routes::post_revealed_secret, // TODO move into own route when cli working
            ],
        )
        .manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_service)
        .manage(bob_success_keypair)
        .manage(bob_refund_address)
        .manage(network)
        .manage(bob_client)
        .manage(Mutex::new(rng))
}
