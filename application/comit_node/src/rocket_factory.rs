use bitcoin_support::Network;
use comit_node_api_client::ApiClient as ExchangeApiClient;
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
    exchange_refund_address: ethereum_support::Address,
    exchange_success_keypair: KeyPair,
    network: Network,
    exchange_client: Arc<ExchangeApiClient>,
) -> rocket::Rocket {
    let rng = OsRng::new().expect("Failed to get randomness from OS");

    rocket::ignite()
        .mount(
            "/",
            routes![
                eth_btc::buy::routes::post_buy_orders,
                eth_btc::buy::routes::post_orders_funding,
                eth_btc::buy::routes::post_revealed_secret,
                eth_btc::sell::routes::post_orders_funding,
                eth_btc::sell::routes::post_revealed_secret,
            ],
        )
        .mount(
            "/cli/", //todo come up with a better name
            routes![
                eth_btc::cli::buy_routes::get_redeem_orders,
                eth_btc::cli::buy_routes::post_buy_offers,
                eth_btc::cli::buy_routes::post_buy_orders,
                eth_btc::cli::sell_routes::post_sell_offers,
                eth_btc::cli::sell_routes::post_sell_orders,
                eth_btc::ledger::routes::post_contract_deployed, // TODO uncomment when cli working
            ],
        )
        .manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_service)
        .manage(exchange_success_keypair)
        .manage(exchange_refund_address)
        .manage(network)
        .manage(exchange_client)
        .manage(Mutex::new(rng))
}
