#[macro_use]
extern crate structopt;
use structopt::StructOpt;
use std::str::FromStr;

#[derive(Debug)]
struct Currency(String);

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(Currency(s.to_string()))
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "Trading Client", about = "CLI for the atomic swap trading service.")]
struct Opt {
    /// The currency you want to sell.
    #[structopt(short = "s", long = "sell")]
    sell: Currency,
    /// The currency you want to buy.
    #[structopt(short = "b", long = "buy")]
    buy: Currency,
    /// The amount you want to sell.
    #[structopt(short = "a", long = "amount")]
    amount: u32,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
}
