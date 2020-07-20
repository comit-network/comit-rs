use std::path::PathBuf;
use structopt::StructOpt;

mod balance;
mod deposit;
mod trade;
mod wallet_info;

pub use balance::*;
pub use deposit::*;
pub use trade::*;
pub use wallet_info::*;

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Path to configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,
    /// Start trading
    #[structopt(subcommand)]
    pub cmd: Command,
}

impl Options {
    pub fn from_args() -> Self {
        StructOpt::from_args()
    }
}

#[derive(StructOpt, Debug, Copy, Clone)]
pub enum Command {
    Trade,
    WalletInfo,
    Balance,
    Deposit,
}
