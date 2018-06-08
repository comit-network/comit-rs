extern crate bitcoin_rpc;
extern crate common_types;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate structopt;
extern crate trading_client;
extern crate uuid;

use std::env::var;
use std::string::String;
use structopt::StructOpt;
use trading_client::offer;
use trading_client::offer::Symbol;
use trading_client::order;
use trading_client::redeem;
use trading_client::redeem::RedeemOutput;
use trading_client::trading_service_api_client::TradingApiUrl;
use uuid::Uuid;

#[derive(Debug, StructOpt)]
#[structopt(name = "Trading Client", about = "CLI for the atomic swap trading service.")]
enum Opt {
    /// Request an offer
    #[structopt(name = "offer")]
    Offer {
        /// The symbol you want to trade (e.g. ETH-BTC)
        #[structopt(short = "S", long = "symbol", name = "symbol to trade (e.g. ETH-BTC)")]
        symbol: String,
        /// Request a buy order
        #[structopt(short = "b", long = "buy", name = "request for a buy order")]
        buy: bool,
        /// Request a sell order
        #[structopt(short = "s", long = "sell", name = "request for a sell order")]
        sell: bool,
        /// The amount you want to exchange (buy for a buy order, sell for a sell order). Integer.
        #[structopt(short = "a", long = "amount",
                    name = "amount to exchange (buy for a buy order, sell for a sell order). Integer.")]
        amount: u32,
    },
    /// Accept an order
    #[structopt(name = "order")]
    Order {
        /// The symbol you want to trade (e.g. ETH-BTC)
        #[structopt(short = "S", long = "symbol", name = "symbol to trade (e.g. ETH-BTC)")]
        symbol: String,
        /// The trade id
        #[structopt(short = "u", long = "uid", name = "trade id")]
        uid: Uuid,
        /// The address to receive the traded currency
        #[structopt(short = "d", long = "success-address",
                    name = "address to receive the traded currency")]
        success_address: String,
        /// The address to receive a refund in the original currency in case the trade is cancelled
        #[structopt(short = "r", long = "refund-address",
                    name = "address to receive your refund in case of cancellation")]
        refund_address: String,
    },
    /// Get details to proceed with redeem transaction
    #[structopt(name = "redeem")]
    Redeem {
        /// The symbol you want to trade (e.g. ETH-BTC)
        #[structopt(short = "S", long = "symbol", name = "symbol to trade (e.g. ETH-BTC)")]
        symbol: String,
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
            symbol,
            buy,
            sell,
            amount,
        } => offer::run(
            trading_api_url,
            Symbol::from(symbol),
            offer::OrderType::new(buy, sell),
            amount,
        ),
        Opt::Order {
            symbol,
            uid,
            success_address,
            refund_address,
        } => order::run(
            trading_api_url,
            Symbol::from(symbol),
            uid,
            success_address,
            refund_address,
        ),
        Opt::Redeem {
            symbol,
            uid,
            console,
        } => redeem::run(
            trading_api_url,
            Symbol::from(symbol),
            uid,
            RedeemOutput::new(console),
        ),
    };

    println!("{}", output.unwrap())
}
