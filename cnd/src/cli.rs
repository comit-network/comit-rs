use crate::LocalSwapId;
use comit::Secret;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Path to configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,

    /// Dump the current configuration and exit
    #[structopt(long = "dump-config")]
    pub dump_config: bool,

    /// Display the current version
    #[structopt(short = "V", long = "version")]
    pub version: bool,

    /// Which network to connect to
    #[structopt(short = "n", long = "network")]
    pub network: Option<comit::Network>,

    /// Commands available
    #[structopt(subcommand)]
    pub cmd: Option<Command>,
}

#[derive(StructOpt, Debug, Clone)]
pub enum Command {
    /// Prints the secret in case this node acts in the role of Alice that was
    /// derived for a specific swap. Bob doesn't get to choose the secret hence
    /// this command will fail for swaps where this node acts as Bob.
    PrintSecret { swap_id: LocalSwapId },
    /// Manually create and sign a transaction for a specific swap.
    CreateTransaction(CreateTransaction),
}

#[derive(StructOpt, Debug, Clone)]
pub enum CreateTransaction {
    /// Create the transaction for the `redeem` action.
    Redeem {
        /// The ID of the swap.
        swap_id: LocalSwapId,
        /// The hex-encoded, 32-byte secret needed to unlock the coins. Only
        /// needed if this node acts in the role of Bob for this swap.
        #[structopt(long, parse(try_from_str = parse_secret))]
        secret: Option<Secret>,
        /// The Bitcoin outpoint where the `hbit` HTLC is located in the form of
        /// `<txid>:<vout>`.
        #[structopt(long)]
        outpoint: ::bitcoin::OutPoint,
        /// The Bitcoin address the coins should be redeemed to.
        #[structopt(long)]
        address: ::bitcoin::Address,
    },
    /// Create the transaction for the `refund` action.
    Refund {
        /// The ID of the swap.
        swap_id: LocalSwapId,
        /// The Bitcoin outpoint where the `hbit` HTLC is located in the form of
        /// `<txid>:<vout>`.
        #[structopt(long)]
        outpoint: ::bitcoin::OutPoint,
        /// The Bitcoin address the coins should be refunded to.
        #[structopt(long)]
        address: ::bitcoin::Address,
    },
}

fn parse_secret(str: &str) -> anyhow::Result<Secret> {
    let mut secret = [0u8; 32];
    hex::decode_to_slice(str, &mut secret)?;

    Ok(Secret::from(secret))
}
