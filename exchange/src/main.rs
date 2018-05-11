#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate bitcoin_rpc;
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

use bitcoin_rpc::types::Address;
use reqwest::Error;
use rocket::response::status::BadRequest;
use rocket::State;
use rocket_contrib::Json;
use std::collections::HashMap;
use std::env::var;
use std::sync::Mutex;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Offer {
    symbol: String,
    rate: f32,
    uid: Uuid,
    address: Address,
}

struct Offers {
    all_offers: Mutex<HashMap<Uuid, Offer>>,
}

lazy_static! {
    static ref TREASURY_SERVICE_URL: String = var("TREASURY_SERVICE_URL").unwrap();
}

fn get_rate(url: &str, offer_request: OfferRequest) -> Result<Rate, Error> {
    reqwest::get(format!("{}/rate/{}", url, offer_request.symbol).as_str())?.json::<Rate>()
}

#[post("/offers", format = "application/json", data = "<offer_request>")]
fn offers_request(
    offers: State<Offers>,
    offer_request: Json<OfferRequest>,
) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request = offer_request.into_inner();

    //TODO: grab TREASURY_SERVICE_URL in the main and add it to the state
    let res = get_rate(&*TREASURY_SERVICE_URL, offer_request);

    match res {
        Ok(rate) => {
            let uid = Uuid::new_v4();

            let offer = Offer {
                symbol: rate.symbol,
                rate: rate.rate,
                uid,
                // TODO: retrieve and use real address
                // This should never be used. Private key is: cSVXkgbkkkjzXV2JMg1zWui4A4dCj55sp9hFoVSUQY9DVh9WWjuj
                address: Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
            };

            let mut result = offers.all_offers.lock().unwrap();
            result.insert(uid, offer);

            let offer = result.get(&uid).unwrap();
            //TODO: avoid the clone
            Ok(Json(offer.clone()))
        }
        Err(e) => {
            error!("{:?}", e);

            Err(BadRequest(None))
        }
    }
}

fn main() {
    let offers = Offers {
        all_offers: Mutex::new(HashMap::new()),
    };

    rocket::ignite()
        .manage(offers)
        .mount("/", routes![self::offers_request])
        .launch();
}
