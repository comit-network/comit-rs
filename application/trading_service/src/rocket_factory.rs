use bitcoin_support::Network;
use event_store::InMemoryEventStore;
use exchange_api_client::ApiClient;
use rand::OsRng;
use rocket;
use std::sync::{Arc, Mutex};
use swaps::{eth_btc, eth_ln, TradeId};

pub fn create_rocket_instance(
    network: Network,
    event_store: InMemoryEventStore<TradeId>,
    client: Arc<ApiClient>,
) -> rocket::Rocket {
    // TODO: allow caller to choose randomness source
    let rng = OsRng::new().expect("Failed to get randomness from OS");
    rocket::ignite()
        .mount(
            "/",
            routes![
                eth_btc::buy::routes::get_redeem_orders,
                eth_btc::buy::routes::post_buy_offers,
                eth_btc::buy::routes::post_buy_orders,
                eth_btc::buy::routes::post_contract_deployed,
                eth_btc::sell::routes::post_sell_offers,
                eth_btc::sell::routes::post_sell_orders,
                eth_btc::sell::routes::post_contract_deployed,
                eth_ln::sell::routes::post_sell_offers,
            ],
        )
        .manage(client)
        .manage(network)
        .manage(event_store)
        .manage(Mutex::new(rng))
}
