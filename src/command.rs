use std::path::PathBuf;
use structopt::StructOpt;

mod balance;
mod deposit;
mod trade;
mod wallet_info;
mod withdraw;

use crate::{
    config::{File, Settings},
    ethereum::{self, dai, ether},
    network::Taker,
    swap::SwapKind,
    {bitcoin, history},
};
use chrono::{DateTime, Local};
use num::BigUint;
use std::str::FromStr;

pub use balance::*;
pub use deposit::*;
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
    /// Start to publish order and execute them
    Trade,
    /// Print all wallets information for backup or export purposes
    WalletInfo,
    /// Print the actual balance on all assets
    Balance,
    /// Print wallet addresses to deposit assets
    Deposit,
    /// Dump the current configuration
    DumpConfig,
    /// Withdraw assets
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

pub fn into_history_trade(
    peer_id: libp2p::PeerId,
    swap: SwapKind,
    #[cfg(not(test))] final_timestamp: DateTime<Local>,
) -> history::Trade {
    use crate::history::*;

    let (swap, position) = match swap {
        SwapKind::HbitHerc20(swap) => (swap, history::Position::Sell),
        SwapKind::Herc20Hbit(swap) => (swap, history::Position::Buy),
    };

    #[cfg(not(test))]
    let final_timestamp = final_timestamp.into();

    #[cfg(test)]
    let final_timestamp = DateTime::from_str("2020-07-10T17:48:26.123+10:00")
        .unwrap()
        .into();

    Trade {
        start_timestamp: history::LocalDateTime::from_utc_naive(&swap.start_of_swap),
        final_timestamp,
        base_symbol: Symbol::Btc,
        quote_symbol: Symbol::Dai,
        position,
        base_precise_amount: swap.hbit_params.shared.asset.as_sat().into(),
        quote_precise_amount: BigUint::from_str(&swap.herc20_params.asset.quantity.to_wei_dec())
            .expect("number to number conversion")
            .into(),
        peer: peer_id.into(),
    }
}

#[derive(Debug, Clone)]
pub struct FinishedSwap {
    pub swap: SwapKind,
    pub taker: Taker,
    pub final_timestamp: DateTime<Local>,
}

impl FinishedSwap {
    pub fn new(swap: SwapKind, taker: Taker, final_timestamp: DateTime<Local>) -> Self {
        Self {
            swap,
            taker,
            final_timestamp,
        }
    }
}
