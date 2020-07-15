use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Path to configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,
    /// Start trading
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt, Debug, Copy, Clone)]
pub enum Command {
    Trade,
}
