use event_store::EventStore;
use rocket;
use routes;

#[derive(Clone)]
pub struct TreasuryApiUrl(pub String);

pub fn create_rocket_instance(
    exchange_api_url: TreasuryApiUrl,
    event_store: EventStore,
) -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![routes::eth_btc::post_buy_offers])
        .mount("/", routes![routes::eth_btc::post_buy_orders])
        .manage(exchange_api_url)
        .manage(event_store)
}
