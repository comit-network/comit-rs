use std::path::PathBuf;

#[derive(structopt::StructOpt, Debug)]
#[structopt(name = "COMIT network daemon")]
pub struct Options {
    /// Path to configuration file.
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,

    /// Path to secret seed file.
    #[structopt(short = "s", long = "seed-file", parse(from_os_str))]
    pub seed_file: Option<PathBuf>,

    /// Dump the current configuration and exit
    #[structopt(short = "d", long = "dump-config")]
    pub dump_config: bool,
}
