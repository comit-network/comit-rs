use bitcoin;
use event_store::EventStore;
use exchange_api_client::ExchangeApiUrl;
use rand::OsRng;
use rocket;
use routes;
use std::sync::Mutex;

pub fn create_rocket_instance(
    exchange_api_url: ExchangeApiUrl,
    network: bitcoin::network::constants::Network,
) -> rocket::Rocket {
    // TODO: allow caller to choose randomness source
    let rng = OsRng::new().expect("Failed to get randomness from OS");
    let event_store = EventStore::new();
    rocket::ignite()
        .mount(
            "/",
            routes![
                routes::eth_btc::get_redeem_orders,
                routes::eth_btc::post_buy_offers,
                routes::eth_btc::post_buy_orders,
                routes::chain_updates::post_update_eth_address,
            ],
        )
        .manage(exchange_api_url)
        .manage(network)
        .manage(event_store)
        .manage(Mutex::new(rng))
}
