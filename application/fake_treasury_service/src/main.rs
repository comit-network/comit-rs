#![feature(plugin, custom_derive)]
#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

use rocket::{http::RawStr, response::status::BadRequest, State};
use rocket_contrib::Json;
use std::{env::var, str::FromStr};

#[derive(Serialize, Deserialize, Debug)]
pub struct RateResponseBody {
    rate: f64,
}

#[get("/<symbol>")]
pub fn get_rates(
    symbol: &RawStr,
    rate: State<f64>,
) -> Result<Json<RateResponseBody>, BadRequest<String>> {
    info!("Rate for {} is {}", symbol, *rate,);
    return Ok(Json(RateResponseBody { rate: *rate }));
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
    use super::*;
    use rocket::http::*;

    #[test]
    fn given_a_rate_then_returned_rate_is_correct() {
        let rate = 0.1;

        let rocket = rocket::ignite()
            .mount("/rates", routes![get_rates])
            .manage(rate);

        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client.get("/rates/ETH-BTC");
        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let rate_response =
            serde_json::from_str::<RateResponseBody>(&response.body_string().unwrap()).unwrap();

        assert_eq!(rate_response.rate, 0.1);
    }
}
