#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate reqwest;

extern crate bitcoin_rpc;
extern crate common_types;
extern crate http_api_problem;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use reqwest::Error;
use rocket_contrib::Json;
use rocket::response::status::*;
use std::env::var;
use uuid::Uuid;
use common_types::TradingSymbol;
use bitcoin_rpc::*;
use common_types::Currency;
use rocket::http::Status;
use http_api_problem::HttpApiProblem;

#[derive(Debug, Deserialize)]
struct Rate {
    symbol: TradingSymbol,
    rate: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct OfferRequest {
    symbol: String,
    sell_amount: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Offer {
    symbol: TradingSymbol,
    rate: f32,
    uid: Uuid,
    target_address: Address,
}

lazy_static! {
    static ref BTC_ADDRESS: Address = Address::from("mjbLRSidW1MY8oubvs4SMEnHNFXxCcoehQ");
    static ref TREASURY_SERVICE_URL: String = var("TREASURY_SERVICE_URL").unwrap();
}

fn get(url: &str, offer_request: OfferRequest) -> Result<Rate, Error> {
    reqwest::get(format!("{}/rate/{}", url, offer_request.symbol).as_str())?.json::<Rate>()
}

#[post("/offers", format = "application/json", data = "<offer_request>")]
fn offers_request(offer_request: Json<OfferRequest>) -> Result<Json<Offer>, HttpApiProblem> {
    let offer_request = offer_request.into_inner();

    let res = get(&*TREASURY_SERVICE_URL, offer_request);

    match res {
        Ok(rate) => {
            let uid = Uuid::new_v4();

            let currency = rate.symbol.first();

            match currency {
                &Currency::BTC => {
                    let offer = Offer {
                        symbol: rate.symbol.clone(),
                        rate: rate.rate,
                        uid,
                        target_address: BTC_ADDRESS.clone(),
                    };

                    //TODO: store uid?

                    Ok(Json(offer))
                }
                _ => {
                    let problem = HttpApiProblem::new("Unsupported currency")
                        .set_status(400)
                        .set_detail(format!("The currency {} is not supported.", currency));

                    Err(problem)
                }
            }
        }
        Err(e) => {
            error!("{:?}", e);

            let problem = HttpApiProblem::new("Quote unavailable")
                .set_status(500)
                .set_detail(format!("Unable to get quote for {}.", offer_request.symbol));

            Err(problem)
        }
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![self::offers_request])
        .launch();
}
