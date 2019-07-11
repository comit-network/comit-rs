use crate::settings::Settings;
use config::ConfigError;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_file: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<Settings, ConfigError> {
    match opt.config_file {
        Some(config_file) => {
            if config_file.exists() {
                Settings::read(config_file)
            } else {
                Err(ConfigError::Message(format!(
                    "Could not load config file: {:?}",
                    config_file
                )))
            }
        }
        None => match directories::UserDirs::new() {
            None => Err(ConfigError::Message(
                "Unable to determine user's home directory".to_string(),
            )),
            Some(dirs) => {
                let user_path_components: PathBuf =
                    [".config", "comit", "btsieve.toml"].iter().collect();
                let config_file = Path::join(dirs.home_dir(), user_path_components);
                log::info!("Config file was not provided - looking up config file in default location at: {:?}", config_file);;
                if config_file.exists() {
                    Settings::read(config_file)
                } else {
                    Err(ConfigError::Message(format!(
                        "Could not load config file: {:?}",
                        config_file
                    )))
                }
            }
        },
    }
}
