#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate reqwest;

extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use reqwest::Error;
use rocket_contrib::Json;
use rocket::response::status::BadRequest;
use std::env::var;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Rate {
    symbol: String,
    rate: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Symbol(String); // Expected format: BTC:LTC

#[derive(Serialize, Deserialize, Debug)]
struct OfferRequest {
    symbol: String,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Offer {
    symbol: String,
    rate: f32,
    uid: Uuid,
}

lazy_static! {
    static ref TREASURY_SERVICE_URL: String = var("TREASURY_SERVICE_URL").unwrap();
}

fn get_rate(url: &str, offer_request: OfferRequest) -> Result<Rate, Error> {
    reqwest::get(format!("{}/rate/{}", url, offer_request.symbol).as_str())?.json::<Rate>()
}

#[post("/offers", format = "application/json", data = "<offer_request>")]
fn offers_request(offer_request: Json<OfferRequest>) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request = offer_request.into_inner();

    let res = get_rate(&*TREASURY_SERVICE_URL, offer_request);

    match res {
        Ok(rate) => {
            let uid = Uuid::new_v4();

            let offer = Offer {
                symbol: rate.symbol,
                rate: rate.rate,
                uid,
            };

            //TODO: store uid?

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
