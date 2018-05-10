#[macro_use]
extern crate structopt;

use std::env::var;
use std::str::FromStr;
use structopt::StructOpt;

extern crate reqwest;
extern crate serde;

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Serialize)]
struct Currency(String);

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(Currency(s.to_string()))
    }
}

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

// TODO implement API
// struct TradingServiceClient {
//     client: reqwest::Client,
//     service_url: String
// }

// trait TradingServiceApi {
//     pub fn create_offer() -> Result<> {

//     }
// }

// struct FakeTradingServiceClient {

// }

fn main() {
    let opt = Opt::from_args();

    println!("{:?}", opt);

    let trading_service_url =
        var("TRADING_SERVICE_URL").unwrap_or_exit("TRADING_SERVICE_URL not set");

    let client = reqwest::Client::new();
    let res = client.post(trading_service_url.as_str()).json(&opt).send();

    println!("{:?}", res);
    // self.client
    //     .json(&request)
    //     .send()
    //     .and_then(|mut res| res.json::<RpcResponse<R>>())
}
