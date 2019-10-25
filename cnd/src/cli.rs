use std::path::PathBuf;

#[derive(structopt::StructOpt, Debug)]
#[structopt(name = "COMIT network daemon")]
pub struct Options {
    /// Path to configuration file.
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config_file: Option<PathBuf>,

    /// Path to secret seed file.
    #[structopt(short = "s", long = "secret-seed-file", parse(from_os_str))]
    pub secret_seed_file: Option<PathBuf>,
}
