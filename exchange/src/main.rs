extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use reqwest::Error;

#[derive(Debug, Deserialize)]
struct Rate {
    sell: String,
    buy: String,
    rate: f32,
}

fn main() {
    let rate = get();
    println!("{:?}", rate);
}

fn get() -> Result<Rate, Error> {
    reqwest::get("http://localhost:8000/rate/btc/eth")?.json::<Rate>()
}
