use bitcoin_support::Network;
use comit_node_api_client::ApiClient;
use ethereum_support;
use event_store::InMemoryEventStore;
use ganp::ledger::{bitcoin::Bitcoin, ethereum::Ethereum};
use rand::OsRng;
use rocket;
use secp256k1_support::KeyPair;
use std::sync::{Arc, Mutex};
use swap_protocols::rfc003::ledger_htlc_service::{
    BitcoinHtlcParams, EtherHtlcParams, LedgerHtlcService,
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
) -> rocket::Rocket {
    let rng = OsRng::new().expect("Failed to get randomness from OS");

    rocket::ignite()
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
        )
        .mount(
            "/ledger/", //Endpoints for notifying about ledger events
            routes![
                // TODO will be removed once we have the Ledger Query Service
                eth_btc::ledger::buy_routes::post_contract_deployed,
                eth_btc::ledger::buy_routes::post_orders_funding,
                eth_btc::ledger::buy_routes::post_revealed_secret,
                eth_btc::ledger::sell_routes::post_orders_funding,
                eth_btc::ledger::sell_routes::post_revealed_secret,
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
