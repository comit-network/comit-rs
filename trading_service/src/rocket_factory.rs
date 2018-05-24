use exchange_api_client::ExchangeApiUrl;
use offer::OfferRepository;
use rand::OsRng;
use rocket;
use routes;
use std::sync::Mutex;

pub fn create_rocket_instance(
    exchange_api_url: ExchangeApiUrl,
    offer_repository: OfferRepository,
) -> rocket::Rocket {
    // TODO: allow caller to choose randomness source
    let rng = OsRng::new().expect("Failed to get randomness from OS");
    rocket::ignite()
        .mount("/", routes![routes::eth_btc::post_buy_offers])
        .manage(exchange_api_url)
        .manage(offer_repository)
        .manage(Mutex::new(rng))
}
