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

    let path = config_dir()
        .map(|dir| Path::join(&dir, "btsieve.toml"))
        .ok_or_else(|| {
            ConfigError::Message("Could not generate default configuration path".to_string())
        })?;

    log::info!(
        "Config file was not provided - looking up config file in default location at: {:?}",
        path
    );

    parse_config_file(path)
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

// Linux: /home/<user>/.config/btsieve/
// Windows: C:\Users\<user>\AppData\Roaming\comit-network\btsieve\config\
// OSX: /Users/<user>/Library/Preferences/comit-network.btsieve/
fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "comit-network", "btsieve")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
}
