#![allow(clippy::print_stdout)] // We cannot use `log` before we have the config file

use crate::settings::Settings;
use config::ConfigError;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Path to configuration folder
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config_file: Option<PathBuf>,
}

pub fn load_settings(opt: Opt) -> Result<Settings, ConfigError> {
    if let Some(file) = opt.config_file {
        println!("Using config file at {:?}", file);
        return parse_config_file(file);
    }

    // Linux: /home/<user>/.config/btsieve
    // Windows: C:\Users\<user>\AppData\Roaming\comit-network\btsieve\config
    // OSX: /Users/<user>/Library/Preferences/comit-network.btsieve
    if let Some(proj_dirs) = ProjectDirs::from("", "comit-network", "btsieve") {
        let file = Path::join(proj_dirs.config_dir(), "btsieve.toml");
        println!(
            "Config file was not provided - looking up config file in default location at: {:?}",
            file
        );
        return parse_config_file(file);
    }

    Err(ConfigError::Message(
        "Could not generate configuration directory".to_string(),
    ))
}

fn parse_config_file(file: PathBuf) -> Result<Settings, ConfigError> {
    if file.exists() {
        Settings::read(file)
    } else {
        Err(ConfigError::Message(format!(
            "Could not load config file: {:?}",
            file
        )))
    }
}
