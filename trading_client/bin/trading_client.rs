#[macro_use]
extern crate structopt;

use std::env::var;
use std::str::FromStr;
use structopt::StructOpt;

extern crate reqwest;
extern crate serde;
extern crate trading_client;

use trading_client::trading_service_api_client::create_client;

#[macro_use]
extern crate serde_derive;

#[derive(Debug, StructOpt, Serialize)]
#[structopt(name = "Trading Client", about = "CLI for the atomic swap trading service.")]
struct Opt {
    /// The currency you want to sell.
    #[structopt(short = "s", long = "sell", name = "currency to sell")]
    sell: Currency,
    /// The currency you want to buy.
    #[structopt(short = "b", long = "buy", name = "currency to buy")]
    buy: Currency,
    /// The amount you want to sell.
    #[structopt(short = "S", long = "sell-amount", name = "amount")]
    sell_amount: u32,
}

trait UnwrapOrExit<T, K> {
    fn unwrap_or_exit(self, msg: &str) -> T;
}

impl<T, E> UnwrapOrExit<T, E> for Result<T, E> {
    fn unwrap_or_exit(self, msg: &str) -> T {
        match self {
            Ok(success) => success,
            Err(_) => {
                eprintln!("{}", msg);
                std::process::exit(1);
            }
        }
    }
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);

    // self.client
    //     .json(&request)
    //     .send()
    //     .and_then(|mut res| res.json::<RpcResponse<R>>())
}
