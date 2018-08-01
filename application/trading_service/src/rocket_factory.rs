use bitcoin_support::Network;
use event_store::InMemoryEventStore;
use events::TradeId;
use exchange_api_client::ApiClient;
use rand::OsRng;
use rocket;
use routes;
use std::sync::{Arc, Mutex};

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
                routes::eth_btc::get_redeem_orders,
                routes::eth_btc::post_buy_offers,
                routes::eth_btc::post_buy_orders,
                routes::eth_btc::post_contract_deployed,
            ],
        )
        .manage(client)
        .manage(network)
        .manage(event_store)
        .manage(Mutex::new(rng))
}
