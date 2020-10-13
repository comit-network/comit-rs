use std::path::PathBuf;

#[derive(structopt::StructOpt, Debug)]
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
}
