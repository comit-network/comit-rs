use std::path::PathBuf;

#[derive(structopt::StructOpt, Debug)]
#[structopt(name = "COMIT network daemon")]
pub struct Options {
    /// Path to configuration file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,
    /// Print version
    #[structopt(long = "version")]
    pub version: bool,
}
