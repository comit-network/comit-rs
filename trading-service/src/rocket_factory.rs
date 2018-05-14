use rocket;
use routes;
use types::ExchangeApiUrl;

pub fn create_rocket_instance(exchange_api_url: ExchangeApiUrl) -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![routes::offers::post])
        .manage(exchange_api_url)
}
