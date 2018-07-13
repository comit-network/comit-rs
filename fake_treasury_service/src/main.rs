#![feature(plugin, custom_derive)]
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate common_types;
extern crate ethereum_support;

use common_types::BitcoinQuantity;
use ethereum_support::EthereumQuantity;
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use std::env::var;
use std::str::FromStr;

#[derive(Deserialize, Debug, FromForm)]
pub struct RateRequestParams {
    amount: f64, //ethereum
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    symbol: String,
    rate: f64,
    sell_amount: BitcoinQuantity,
    buy_amount: EthereumQuantity,
}

fn calculate_sell_amount(buy_amount: f64, rate: f64) -> BitcoinQuantity {
    BitcoinQuantity::from_bitcoin(buy_amount * rate)
}

#[get("/<symbol>?<rate_request_params>")]
pub fn get_rates(
    symbol: &RawStr,
    rate: State<f64>,
    rate_request_params: RateRequestParams,
) -> Result<Json<RateResponseBody>, BadRequest<String>> {
    let buy_amount_eth = rate_request_params.amount;
    let symbol = symbol.to_string();
    let sell_amount = calculate_sell_amount(buy_amount_eth, *rate);
    let buy_amount = EthereumQuantity::from_eth(buy_amount_eth);
    info!(
        "Rate for {} is {}: {}:{}",
        symbol, *rate, buy_amount, sell_amount
    );
    return Ok(Json(RateResponseBody {
        symbol,
        rate: *rate,
        sell_amount,
        buy_amount,
    }));
}

fn main() {
    let rate_str = var("RATE").expect("RATE not set");
    let rate = f64::from_str(&rate_str).expect("RATE wasn't a valid floating point number");

    rocket::ignite()
        .mount("/rates", routes![get_rates])
        .manage(rate)
        .launch();
}

#[cfg(test)]
mod test {
    extern crate serde_json;
    use self::ethereum_support::*;
    use super::*;
    use rocket::http::*;

    #[test]
    fn given_a_rate_and_buy_amount_sell_amount_is_correct_in_wei() {
        let rate = 0.1;

        let rocket = rocket::ignite()
            .mount("/rates", routes![get_rates])
            .manage(rate);

        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client.get("/rates/ETH-BTC?amount=10");
        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let rate_response =
            serde_json::from_str::<RateResponseBody>(&response.body_string().unwrap()).unwrap();

        assert_eq!(rate_response.sell_amount.satoshi(), 100_000_000);
        assert_eq!(
            rate_response.buy_amount.wei(),
            U256::from(10_000_000_000_000_000_000 as u64)
        );
    }
}
