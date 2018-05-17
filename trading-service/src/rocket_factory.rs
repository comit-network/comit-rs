use rand::OsRng;
use rocket;
use routes;
use std::sync::Mutex;
use types::{ExchangeApiUrl, Offers};

pub fn create_rocket_instance(exchange_api_url: ExchangeApiUrl, offers: Offers) -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![routes::offers::post])
        .manage(exchange_api_url)
        .manage(offers)
        .manage(Mutex::new(
            // TODO: allow caller to choose randomness source
            OsRng::new().expect("Failed to get randomness from OS"),
        ))
}
