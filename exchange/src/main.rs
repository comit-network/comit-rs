#[macro_use]
extern crate lazy_static;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use reqwest::Error;
use std::env::var;

#[derive(Debug, Deserialize)]
struct Rate {
    sell: String,
    buy: String,
    rate: f32,
}

lazy_static! {
    static ref TREASURY_SERVICE_URL: String = var("TREASURY_SERVICE_URL").unwrap();
}

fn main() {
    let rate = get(&*TREASURY_SERVICE_URL);
    println!("{:?}", rate);
}

fn get(url: &str) -> Result<Rate, Error> {
    reqwest::get(format!("{}/rate/btc/eth", url).as_str())?.json::<Rate>()
}
