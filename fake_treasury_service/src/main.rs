#![feature(plugin, custom_derive)]
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;

#[derive(Serialize, Deserialize, Debug, FromForm)]
pub struct RateRequestParams {
    //TODO: make it work with float
    amount: u64, //ethereum
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    symbol: String,
    rate: f64,
    sell_amount: u64, //satoshis
    buy_amount: u64,  //ethereum
}

#[get("/<symbol>?<rate_request_params>")]
pub fn get_rates(
    symbol: &RawStr,
    rate_request_params: RateRequestParams,
) -> Result<Json<RateResponseBody>, BadRequest<String>> {
    let buy_amount = rate_request_params.amount;
    let symbol = symbol.to_string();
    let rate = 0.075112;
    let sell_amount = (buy_amount as f64 * rate) * 100_000_000 as f64;
    let sell_amount = sell_amount as u64;

    info!(
        "Rate for {} is {}: {}:{}",
        symbol, rate, buy_amount, sell_amount
    );
    return Ok(Json(RateResponseBody {
        symbol,
        rate,
        sell_amount,
        buy_amount,
    }));
}

fn main() {
    rocket::ignite()
        .mount("/rates", routes![get_rates])
        .launch();
}
