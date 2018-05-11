#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

use rocket_contrib::Json;
use rocket::response::status::BadRequest;
use std::env::var;

#[derive(Serialize, Deserialize)]
struct Symbol(String); // Expected format: BTC:LTC

#[derive(Serialize, Deserialize)]
struct OfferRequest {
    symbol: Symbol,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize)]
struct Offer {
    symbol: Symbol,
    rate: f32,
    uid: String,
}

lazy_static! {
    static ref EXCHANGE_SERVICE_URL: String = var("EXCHANGE_SERVICE_URL").unwrap();
}

#[post("/offers", format = "application/json", data = "<offer_request>")]
fn offers_request(offer_request: Json<OfferRequest>) -> Result<Json<Offer>, BadRequest<String>> {
    let client = reqwest::Client::new();
    let offer_request = offer_request.into_inner();

    let res = client
        .post(&*EXCHANGE_SERVICE_URL)
        .json(&offer_request)
        .send()
        .and_then(|mut res| res.json::<Offer>());

    match res {
        Ok(offer) => {
            // TODO store in database

            Ok(Json(offer))
        }
        Err(e) => {
            error!("{:?}", e);

            Err(BadRequest(None))
        }
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![self::offers_request])
        .launch();
}
