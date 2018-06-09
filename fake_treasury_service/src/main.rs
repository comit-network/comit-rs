#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;

#[derive(Serialize, Deserialize, Debug)]
pub struct RateRequestBody {
    //TODO: make it work with float
    buy_amount: u64, //ethereum
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    symbol: String,
    rate: f64,
    sell_amount: u64,
    //satoshis
    buy_amount: u64, //ethereum
}

#[post("/<symbol>", format = "application/json", data = "<rate_request_body>")]
pub fn post_rates(
    symbol: &RawStr,
    rate_request_body: Json<RateRequestBody>,
) -> Result<Json<RateResponseBody>, BadRequest<String>> {
    let symbol = symbol.to_string();
    let rate_request_body: RateRequestBody = rate_request_body.into_inner();
    let rate = 0.7;
    let buy_amount = rate_request_body.buy_amount;
    let sell_amount = (buy_amount as f64 * rate).round().abs() as u64;
    Ok(Json(RateResponseBody {
        symbol,
        rate,
        sell_amount,
        buy_amount,
    }))
}

fn main() {
    rocket::ignite()
        .mount("/rates", routes![post_rates])
        .launch();
}
