extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate structopt;
extern crate trading_client;
extern crate uuid;

use std::env::var;
use std::str::FromStr;
use structopt::StructOpt;
use trading_client::types::Currency;
use uuid::Uuid;

use trading_client::trading_service_api_client::create_client;

#[macro_use]
extern crate serde_derive;

#[derive(Debug, StructOpt)]
#[structopt(name = "Trading Client", about = "CLI for the atomic swap trading service.")]
enum Opt {
    /// Request an offer
    #[structopt(name = "offer")]
    Offer {
        /// The currency you want to sell.
        #[structopt(short = "s", long = "sell", name = "currency to sell")]
        sell: Currency,
        /// The currency you want to buy.
        #[structopt(short = "b", long = "buy", name = "currency to buy")]
        buy: Currency,
        /// The amount you want to sell.
        #[structopt(short = "S", long = "sell-amount", name = "amount")]
        sell_amount: u32,
    },
    /// Get details to proceed with redeem transaction
    #[structopt(name = "redeem")]
    Redeem {
        /// The trade id
        #[structopt(short = "u", long = "uid", name = "trade id")]
        uid: Uuid,
    },
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
