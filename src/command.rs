use std::path::PathBuf;
use structopt::StructOpt;

mod balance;
mod deposit;
mod trade;
mod wallet_info;
mod withdraw;

use crate::bitcoin;
use crate::config::{File, Settings};
use crate::ethereum::{self, dai, ether};
pub use balance::*;
pub use deposit::*;
use std::str::FromStr;
pub use trade::*;
pub use wallet_info::*;
pub use withdraw::*;

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Path to configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,
    /// Commands available
    #[structopt(subcommand)]
    pub cmd: Command,
}

impl Options {
    pub fn from_args() -> Self {
        StructOpt::from_args()
    }
}

#[derive(StructOpt, Debug, Clone)]
pub enum Command {
    Trade,
    WalletInfo,
    Balance,
    Deposit,
    DumpConfig,
    Withdraw(Withdraw),
}

pub fn dump_config(settings: Settings) -> anyhow::Result<()> {
    let file = File::from(settings);
    let serialized = toml::to_string(&file)?;
    println!("{}", serialized);
    Ok(())
}

// TODO: This takes the nominal amount (ether, bitcoin, dai)
// We could add more option to accept the smallest unit (wei, sats, attodai)
#[derive(StructOpt, Debug, Clone)]
pub enum Withdraw {
    Btc {
        #[structopt(parse(try_from_str = parse_bitcoin))]
        amount: bitcoin::Amount,
        to_address: bitcoin::Address,
    },
    Dai {
        #[structopt(parse(try_from_str = parse_dai))]
        amount: dai::Amount,
        to_address: ethereum::Address,
    },
    Eth {
        #[structopt(parse(try_from_str = parse_ether))]
        amount: ether::Amount,
        to_address: ethereum::Address,
    },
}

fn parse_bitcoin(str: &str) -> anyhow::Result<bitcoin::Amount> {
    // TODO: In addition to providing an interface to withdraw satoshi, we could use string instead of
    // float here
    let btc = f64::from_str(str)?;
    bitcoin::Amount::from_btc(btc)
}

fn parse_dai(str: &str) -> anyhow::Result<dai::Amount> {
    // TODO: In addition to providing an interface to withdraw attodai, we could use string instead of
    // float here
    let dai = f64::from_str(str)?;
    dai::Amount::from_dai_trunc(dai)
}

fn parse_ether(str: &str) -> anyhow::Result<ether::Amount> {
    ether::Amount::from_ether_str(str)
}
