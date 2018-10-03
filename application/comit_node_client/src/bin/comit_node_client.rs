#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
extern crate bitcoin_support;
extern crate comit_node_client;
extern crate ethereum_support;
extern crate reqwest;
extern crate structopt;
#[macro_use]
extern crate maplit;

use comit_node_client::api_client::{
    ApiClient, Asset, ComitNodeApiUrl, DefaultApiClient, Ledger, SwapRequest, SwapStatus, TradeId,
};
use std::{collections::HashMap, env::var, str::FromStr, string::String};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Comit Node Client",
    about = "CLI for the COMIT Node."
)]
enum Opt {
    /// Sends a Swap request
    #[structopt(name = "swap")]
    Swap {
        #[structopt(subcommand)]
        swap_command: SwapCommand,
    },
}

fn parse_eth_addr(address: &str) -> Result<ethereum_support::Address, String> {
    if !address.starts_with("0x") {
        Err("Ethereum addresses must start with 0x".into())
    } else {
        ethereum_support::Address::from_str(address.trim_left_matches("0x"))
            .map_err(|_| "Invalid ethereum address".to_string())
    }
}

#[derive(StructOpt, Debug)]
enum SwapCommand {
    /// Swap Bitcoin for Ether
    #[structopt(name = "btc-eth")]
    BtcEth {
        /// The amount of Bitcoin to SELL
        #[structopt(name = "BTC")]
        btc_quantity: bitcoin_support::BitcoinQuantity,
        /// The amount of Ethereum to BUY
        #[structopt(name = "ETH")]
        eth_quantity: ethereum_support::EthereumQuantity,

        /// The refund address
        #[structopt(name = "Bitcoin Address")]
        btc_addr: bitcoin_support::Address,

        /// The redemption address
        #[structopt(
            name = "Ethereum Address",
            parse(try_from_str = "parse_eth_addr")
        )]
        eth_addr: ethereum_support::Address,
    },

    /// Display the state of an atomic swap
    #[structopt(name = "status")]
    SwapState {
        #[structopt(name = "ID")]
        id: TradeId,
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
    let opts = Opt::from_args();

    let trading_api_url = ComitNodeApiUrl(
        var("COMIT_NODE_URL").unwrap_or_exit("env variable COMIT_NODE_URL must be set"),
    );

    let client = DefaultApiClient {
        url: trading_api_url,
        client: reqwest::Client::new(),
    };

    match opts {
        Opt::Swap {
            swap_command:
                SwapCommand::BtcEth {
                    btc_quantity,
                    eth_quantity,
                    btc_addr,
                    eth_addr,
                },
        } => {
            let request = SwapRequest {
                source_ledger: Ledger {
                    value: "Bitcoin".to_string(),
                    identity: format!("{:x}", bitcoin_support::PubkeyHash::from(btc_addr)),
                    parameters: HashMap::new(),
                },
                target_ledger: Ledger {
                    value: "Ethereum".to_string(),
                    identity: format!("0x{:x}", eth_addr),
                    parameters: HashMap::new(),
                },
                source_asset: Asset {
                    value: "Bitcoin".to_string(),
                    parameters: convert_args!(hashmap!(
                        "quantity" =>  format!("{}", btc_quantity.satoshi())
                    )),
                },
                target_asset: Asset {
                    value: "Ether".to_string(),
                    parameters: convert_args!(hashmap!(
                        "quantity" => format!("{}", eth_quantity.wei())
                    )),
                },
            };

            let response = client.send_swap_request(request);

            match response {
                Ok(swap_created) => println!("{}", swap_created.id),
                Err(e) => {
                    eprintln!("{:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Opt::Swap {
            swap_command: SwapCommand::SwapState { id },
        } => {
            let response = client.get_swap_status(id);
            match response {
                Ok(swap_status) => {
                    use SwapStatus::*;
                    match swap_status {
                        Pending => {
                            println!("status: pending");
                        }
                        Accepted { funding_required } => {
                            println!("status: accepted");
                            println!("funding_required: {}", funding_required);
                        }
                        Redeemable {
                            contract_address,
                            data,
                            gas,
                        } => {
                            println!("status: redeemable");
                            println!("contract_address: {}", contract_address);
                            println!("data: {}", data);
                            println!("gas: {}", gas);
                        }
                        _ => unimplemented!(),
                    }
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
