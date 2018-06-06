extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate structopt;
extern crate trading_client;
extern crate uuid;

use std::env::var;
use structopt::StructOpt;
use trading_client::offer::Currency;
use trading_client::redeem;
use trading_client::types::TradingApiUrl;
use uuid::Uuid;

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
        /// The amount you want to buy.
        #[structopt(short = "a", long = "buy-amount", name = "amount to buy (integer)")]
        buy_amount: u32,
    },
    /// Get details to proceed with redeem transaction
    #[structopt(name = "redeem")]
    Redeem {
        /// The trade id
        #[structopt(short = "u", long = "uid", name = "trade id")]
        uid: Uuid,
        #[structopt(short = "c", long = "console", name = "web3 console format")]
        console: bool,
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
    let trading_api_url = TradingApiUrl(var("TRADING_SERVICE_URL").unwrap());

    let output = match Opt::from_args() {
        Opt::Offer {
            sell: _,
            buy: _,
            buy_amount: _,
        } => unimplemented!(),
        Opt::Redeem { uid, console } => redeem::run(trading_api_url, uid, output_type(console)),
    };

    println!("{}", output.unwrap())
}

fn output_type(console: bool) -> redeem::OutputType {
    if console {
        redeem::OutputType::CONSOLE
    } else {
        redeem::OutputType::URL
    }
}
