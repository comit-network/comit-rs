use crate::settings::Settings;
use config::ConfigError;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_file: PathBuf,
}

pub fn load_settings(opt: Opt) -> Result<Settings, ConfigError> {
    if opt.config_file.exists() {
        Settings::read(opt.config_file)
    } else {
        Err(ConfigError::Message(format!(
            "Could not load config file: {:?}",
            opt.config_file
        )))
    }
}
