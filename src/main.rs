#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::dbg_macro
)]
#![allow(dead_code)] // To be removed further down the line
#![forbid(unsafe_code)]
#![recursion_limit = "256"]
// TODO: Add no unwrap policy

mod bitcoin;
mod command;
mod config;
mod ethereum;
mod float_maths;
mod fs;
mod history;
mod jsonrpc;
mod maker;
mod mid_market_rate;
mod network;
mod order;
mod rate;
mod seed;
mod swap;
mod swap_id;
mod trace;

#[cfg(all(test, feature = "test-docker"))]
mod test_harness;

use crate::{
    command::{balance, deposit, dump_config, trade, wallet_info, withdraw, Command, Options},
    config::Settings,
};
use anyhow::Context;
use conquer_once::Lazy;
pub use maker::Maker;
pub use mid_market_rate::MidMarketRate;
pub use rate::{Rate, Spread};
pub use seed::Seed;
pub use swap_id::SwapId;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);

#[tokio::main]
async fn main() {
    let options = Options::from_args();

    let settings = read_config(&options)
        .and_then(Settings::from_config_file_and_defaults)
        .expect("Could not initialize configuration");

    if let Command::DumpConfig = options.cmd {
        dump_config(settings).unwrap();
        std::process::exit(0);
    }

    trace::init_tracing(settings.logging.level).unwrap();

    let seed = config::Seed::from_file_or_generate(&settings.data.dir)
        .expect("Could not retrieve/initialize seed")
        .into();

    let dai_contract_addr = settings.ethereum.dai_contract_address;

    let bitcoin_wallet = bitcoin::Wallet::new(
        seed,
        settings.bitcoin.bitcoind.node_url.clone(),
        settings.bitcoin.network,
    )
    .await;
    let ethereum_wallet = ethereum::Wallet::new(
        seed,
        settings.ethereum.node_url.clone(),
        dai_contract_addr.into(),
        settings.ethereum.chain_id,
    )
    .await;

    match options.cmd {
        Command::Trade => {
            let runtime = tokio::runtime::Runtime::new().unwrap();

            trade(
                runtime.handle().clone(),
                &seed,
                settings,
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
                ethereum_wallet.expect("could not initialise ethereum wallet"),
            )
            .await
            .expect("Start trading")
        }
        Command::WalletInfo => {
            let wallet_info = wallet_info(
                ethereum_wallet.ok(),
                bitcoin_wallet.ok(),
                &seed,
                settings.bitcoin.network,
            )
            .await
            .unwrap();
            println!("{}", wallet_info);
        }
        Command::Balance => {
            let balance = balance(
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
            )
            .await
            .unwrap();
            println!("{}", balance);
        }
        Command::Deposit => {
            let deposit = deposit(
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
            )
            .await
            .unwrap();
            println!("{}", deposit);
        }
        Command::Withdraw(arguments) => {
            let tx_id = withdraw(
                ethereum_wallet.expect("could not initialise ethereum wallet"),
                bitcoin_wallet.expect("could not initialise bitcoin wallet"),
                arguments,
            )
            .await
            .unwrap();
            println!("Withdraw successful. Transaction Id: {}", tx_id);
        }
        Command::DumpConfig => unreachable!(),
    }
}

fn read_config(options: &Options) -> anyhow::Result<config::File> {
    // if the user specifies a config path, use it
    if let Some(path) = &options.config_file {
        eprintln!("Using config file {}", path.display());

        return config::File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    // try to load default config
    let default_path = crate::fs::default_config_path()?;

    if !default_path.exists() {
        return Ok(config::File::default());
    }

    eprintln!(
        "Using config file at default path: {}",
        default_path.display()
    );

    config::File::read(&default_path)
        .with_context(|| format!("failed to read config file {}", default_path.display()))
}
