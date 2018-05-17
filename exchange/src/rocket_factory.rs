use rocket;
use routes;
use types::{Offers, TreasuryApiUrl};

pub fn create_rocket_instance(exchange_api_url: TreasuryApiUrl, offers: Offers) -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![routes::eth_btc::post])
        .manage(exchange_api_url)
        .manage(offers)
}
