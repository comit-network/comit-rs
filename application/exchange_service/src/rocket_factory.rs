use bitcoin_support::Network;
use common_types::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use ethereum_support;
use event_store::InMemoryEventStore;
use ledger_htlc_service::LedgerHtlcService;
use rocket;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swaps::{common::TradeId, eth_btc};
use treasury_api_client::ApiClient;

pub fn create_rocket_instance(
    treasury_api_client: Arc<ApiClient>,
    event_store: InMemoryEventStore<TradeId>,
    ethereum_service: Arc<LedgerHtlcService<Ethereum>>,
    bitcoin_service: Arc<LedgerHtlcService<Bitcoin>>,
    exchange_refund_address: ethereum_support::Address,
    exchange_success_keypair: KeyPair,
    network: Network,
) -> rocket::Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes![
                eth_btc::buy::routes::post_buy_offers,
                eth_btc::buy::routes::post_buy_orders,
                eth_btc::buy::routes::post_orders_funding,
                eth_btc::buy::routes::post_revealed_secret,
                eth_btc::sell::routes::post_orders_funding,
                eth_btc::sell::routes::post_revealed_secret,
            ],
        )
        .manage(treasury_api_client)
        .manage(event_store)
        .manage(ethereum_service)
        .manage(bitcoin_service)
        .manage(exchange_success_keypair)
        .manage(exchange_refund_address)
        .manage(network)
}
